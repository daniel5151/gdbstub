use core::marker::PhantomData;

#[cfg(feature = "alloc")]
use alloc::collections::BTreeMap;

use managed::ManagedSlice;

use crate::common::*;
use crate::{
    arch::{Arch, RegId, Registers},
    connection::Connection,
    internal::*,
    protocol::{
        commands::{ext, Command},
        ConsoleOutput, IdKind, Packet, ResponseWriter, ThreadId,
    },
    target::ext::base::multithread::{Actions, ResumeAction, ThreadStopReason, TidSelector},
    target::ext::base::BaseOps,
    target::Target,
    util::managed_vec::ManagedVec,
    FAKE_PID, SINGLE_THREAD_TID,
};

mod builder;
mod error;
mod target_result_ext;

pub use builder::{GdbStubBuilder, GdbStubBuilderError};
pub use error::GdbStubError;

use target_result_ext::TargetResultExt;

use GdbStubError as Error;

/// Describes why the GDB session ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DisconnectReason {
    /// Target Halted
    TargetHalted,
    /// GDB issued a disconnect command
    Disconnect,
    /// GDB issued a kill command
    Kill,
}

/// Debug a [`Target`] using the GDB Remote Serial Protocol over a given
/// [`Connection`].
pub struct GdbStub<'a, T: Target, C: Connection> {
    conn: C,
    packet_buffer: ManagedSlice<'a, u8>,
    state: GdbStubImpl<T, C>,
}

impl<'a, T: Target, C: Connection> GdbStub<'a, T, C> {
    /// Create a [`GdbStubBuilder`] using the provided Connection.
    pub fn builder(conn: C) -> GdbStubBuilder<'a, T, C> {
        GdbStubBuilder::new(conn)
    }

    /// Create a new `GdbStub` using the provided connection.
    ///
    /// For fine-grained control over various `GdbStub` options, use the
    /// [`builder()`](GdbStub::builder) method instead.
    ///
    /// _Note:_ `new` is only available when the `alloc` feature is enabled.
    #[cfg(feature = "alloc")]
    pub fn new(conn: C) -> GdbStub<'a, T, C> {
        GdbStubBuilder::new(conn).build().unwrap()
    }

    /// Starts a GDB remote debugging session.
    ///
    /// Returns once the GDB client closes the debugging session, or if the
    /// target halts.
    pub fn run(&mut self, target: &mut T) -> Result<DisconnectReason, Error<T::Error, C::Error>> {
        self.state
            .run(target, &mut self.conn, &mut self.packet_buffer)
    }
}

struct GdbStubImpl<T: Target, C: Connection> {
    _target: PhantomData<T>,
    _connection: PhantomData<C>,

    packet_buffer_len: usize,
    current_mem_tid: Tid,
    current_resume_tid: TidSelector,
    no_ack_mode: bool,

    // Used to track which Pids were attached to / spawned when running in extended mode.
    //
    // An empty `BTreeMap<Pid, bool>` is only 24 bytes (on 64-bit systems), and doesn't allocate
    // until the first element is inserted, so it should be fine to include it as part of the main
    // state structure whether or not extended mode is actually being used.
    #[cfg(feature = "alloc")]
    attached_pids: BTreeMap<Pid, bool>,
}

enum HandlerStatus {
    Handled,
    NeedsOK,
    Disconnect(DisconnectReason),
}

impl<T: Target, C: Connection> GdbStubImpl<T, C> {
    fn new(packet_buffer_len: usize) -> GdbStubImpl<T, C> {
        GdbStubImpl {
            _target: PhantomData,
            _connection: PhantomData,

            packet_buffer_len,
            // HACK: current_mem_tid is immediately updated with valid value once `run` is called.
            // While the more idiomatic way to handle this would be to use an Option, given that
            // it's only ever unset prior to the start of `run`, it's probably okay leaving it as-is
            // for code-clarity purposes.
            current_mem_tid: SINGLE_THREAD_TID,
            current_resume_tid: TidSelector::All,
            no_ack_mode: false,

            #[cfg(feature = "alloc")]
            attached_pids: BTreeMap::new(),
        }
    }

    fn run(
        &mut self,
        target: &mut T,
        conn: &mut C,
        packet_buffer: &mut ManagedSlice<u8>,
    ) -> Result<DisconnectReason, Error<T::Error, C::Error>> {
        conn.on_session_start().map_err(Error::ConnectionRead)?;

        // before even accepting packets, we query the target to get a sane value for
        // `self.current_mem_tid`.
        // NOTE: this will break if extended mode is ever implemented...

        self.current_mem_tid = match target.base_ops() {
            BaseOps::SingleThread(_) => SINGLE_THREAD_TID,
            BaseOps::MultiThread(ops) => {
                let mut first_tid = None;
                ops.list_active_threads(&mut |tid| {
                    if first_tid.is_none() {
                        first_tid = Some(tid);
                    }
                })
                .map_err(Error::TargetError)?;
                first_tid.ok_or(Error::NoActiveThreads)?
            }
        };

        loop {
            match Self::recv_packet(conn, target, packet_buffer)? {
                Packet::Ack => {}
                Packet::Nack => return Err(Error::ClientSentNack),
                Packet::Interrupt => {
                    debug!("<-- interrupt packet");
                    let mut res = ResponseWriter::new(conn);
                    res.write_str("S05")?;
                    res.flush()?;
                }
                Packet::Command(command) => {
                    // Acknowledge the command
                    if !self.no_ack_mode {
                        conn.write(b'+').map_err(Error::ConnectionRead)?;
                    }

                    let mut res = ResponseWriter::new(conn);
                    let disconnect = match self.handle_command(&mut res, target, command) {
                        Ok(HandlerStatus::Handled) => None,
                        Ok(HandlerStatus::NeedsOK) => {
                            res.write_str("OK")?;
                            None
                        }
                        Ok(HandlerStatus::Disconnect(reason)) => Some(reason),
                        // HACK: handling this "dummy" error is required as part of the
                        // `TargetResultExt::handle_error()` machinery.
                        Err(Error::NonFatalError(code)) => {
                            res.write_str("E")?;
                            res.write_num(code)?;
                            None
                        }
                        Err(Error::TargetError(e)) => {
                            // unlike all other errors which are "unrecoverable" in the sense that
                            // the GDB session cannot continue, there's still a chance that a target
                            // might want to keep the debugging session alive to do a "post-mortem"
                            // analysis. As such, we simply report a standard TRAP stop reason.
                            let mut res = ResponseWriter::new(conn);
                            res.write_str("S05")?;
                            res.flush()?;
                            return Err(Error::TargetError(e));
                        }
                        Err(e) => return Err(e),
                    };

                    // HACK: this could be more elegant...
                    if disconnect != Some(DisconnectReason::Kill) {
                        res.flush()?;
                    }

                    if let Some(disconnect_reason) = disconnect {
                        return Ok(disconnect_reason);
                    }
                }
            };
        }
    }

    fn recv_packet<'a>(
        conn: &mut C,
        target: &mut T,
        pkt_buf: &'a mut ManagedSlice<u8>,
    ) -> Result<Packet<'a>, Error<T::Error, C::Error>> {
        let header_byte = conn.read().map_err(Error::ConnectionRead)?;

        // Wrap the buf in a `ManagedVec` to keep the code readable.
        let mut buf = ManagedVec::new(pkt_buf);

        buf.clear();
        buf.push(header_byte)?;
        if header_byte == b'$' {
            // read the packet body
            loop {
                let c = conn.read().map_err(Error::ConnectionRead)?;
                buf.push(c)?;
                if c == b'#' {
                    break;
                }
            }
            // read the checksum as well
            buf.push(conn.read().map_err(Error::ConnectionRead)?)?;
            buf.push(conn.read().map_err(Error::ConnectionRead)?)?;
        }

        match Packet::from_buf(target, pkt_buf.as_mut()) {
            Ok(packet) => Ok(packet),
            Err(e) => Err(Error::PacketParse(e)),
        }
    }

    fn handle_command(
        &mut self,
        res: &mut ResponseWriter<C>,
        target: &mut T,
        cmd: Command<'_>,
    ) -> Result<HandlerStatus, Error<T::Error, C::Error>> {
        match cmd {
            Command::Unknown(cmd) => {
                info!("Unknown command: {}", cmd);
                Ok(HandlerStatus::Handled)
            }
            Command::Base(cmd) => self.handle_base(res, target, cmd),
            Command::ExtendedMode(cmd) => self.handle_extended_mode(res, target, cmd),
            Command::MonitorCmd(cmd) => self.handle_monitor_cmd(res, target, cmd),
            Command::SectionOffsets(cmd) => self.handle_section_offsets(res, target, cmd),
        }
    }

    fn handle_base<'a>(
        &mut self,
        res: &mut ResponseWriter<C>,
        target: &mut T,
        command: ext::Base<'a>,
    ) -> Result<HandlerStatus, Error<T::Error, C::Error>> {
        let handler_status = match command {
            // ------------------ Handshaking and Queries ------------------- //
            ext::Base::qSupported(cmd) => {
                // XXX: actually read what the client supports, and enable/disable features
                // appropriately
                let _features = cmd.features.into_iter();

                res.write_str("PacketSize=")?;
                res.write_num(self.packet_buffer_len)?;

                res.write_str(";vContSupported+")?;
                res.write_str(";multiprocess+")?;
                res.write_str(";QStartNoAckMode+")?;

                if let Some(ops) = target.extended_mode() {
                    if ops.configure_aslr().is_some() {
                        res.write_str(";QDisableRandomization+")?;
                    }

                    if ops.configure_env().is_some() {
                        res.write_str(";QEnvironmentHexEncoded+")?;
                        res.write_str(";QEnvironmentUnset+")?;
                        res.write_str(";QEnvironmentReset+")?;
                    }

                    if ops.configure_startup_shell().is_some() {
                        res.write_str(";QStartupWithShell+")?;
                    }

                    if ops.configure_working_dir().is_some() {
                        res.write_str(";QSetWorkingDir+")?;
                    }
                }

                res.write_str(";swbreak+")?;
                if target.hw_breakpoint().is_some() || target.hw_watchpoint().is_some() {
                    res.write_str(";hwbreak+")?;
                }

                // TODO: implement conditional breakpoint support (since that's kool).
                // res.write_str("ConditionalBreakpoints+;")?;

                if T::Arch::target_description_xml().is_some()
                    || target.target_description_xml_override().is_some()
                {
                    res.write_str(";qXfer:features:read+")?;
                }

                HandlerStatus::Handled
            }
            ext::Base::QStartNoAckMode(_) => {
                self.no_ack_mode = true;
                HandlerStatus::NeedsOK
            }
            ext::Base::qXferFeaturesRead(cmd) => {
                #[allow(clippy::redundant_closure)]
                let xml = target
                    .target_description_xml_override()
                    .map(|ops| ops.target_description_xml())
                    .or_else(|| T::Arch::target_description_xml());

                match xml {
                    Some(xml) => {
                        let xml = xml.trim();
                        if cmd.offset >= xml.len() {
                            // no more data
                            res.write_str("l")?;
                        } else if cmd.offset + cmd.len >= xml.len() {
                            // last little bit of data
                            res.write_str("l")?;
                            res.write_binary(&xml.as_bytes()[cmd.offset..])?
                        } else {
                            // still more data
                            res.write_str("m")?;
                            res.write_binary(&xml.as_bytes()[cmd.offset..(cmd.offset + cmd.len)])?
                        }
                    }
                    // If the target hasn't provided their own XML, then the initial response to
                    // "qSupported" wouldn't have included  "qXfer:features:read", and gdb wouldn't
                    // send this packet unless it was explicitly marked as supported.
                    None => return Err(Error::PacketUnexpected),
                }
                HandlerStatus::Handled
            }

            // -------------------- "Core" Functionality -------------------- //
            // TODO: Improve the '?' response based on last-sent stop reason.
            ext::Base::QuestionMark(_) => {
                res.write_str("S05")?;
                HandlerStatus::Handled
            }
            ext::Base::qAttached(cmd) => {
                let is_attached = match target.extended_mode() {
                    // when _not_ running in extended mode, just report that we're attaching to an
                    // existing process.
                    None => true, // assume attached to an existing process
                    // When running in extended mode, we must defer to the target
                    Some(ops) => {
                        let pid: Pid = cmd.pid.ok_or(Error::PacketUnexpected)?;

                        #[cfg(feature = "alloc")]
                        {
                            let _ = ops; // doesn't actually query the target
                            *self.attached_pids.get(&pid).unwrap_or(&true)
                        }

                        #[cfg(not(feature = "alloc"))]
                        {
                            ops.query_if_attached(pid).handle_error()?.was_attached()
                        }
                    }
                };
                res.write_str(if is_attached { "1" } else { "0" })?;
                HandlerStatus::Handled
            }
            ext::Base::g(_) => {
                let mut regs: <T::Arch as Arch>::Registers = Default::default();
                match target.base_ops() {
                    BaseOps::SingleThread(ops) => ops.read_registers(&mut regs),
                    BaseOps::MultiThread(ops) => {
                        ops.read_registers(&mut regs, self.current_mem_tid)
                    }
                }
                .handle_error()?;

                let mut err = Ok(());
                regs.gdb_serialize(|val| {
                    let res = match val {
                        Some(b) => res.write_hex_buf(&[b]),
                        None => res.write_str("xx"),
                    };
                    if let Err(e) = res {
                        err = Err(e);
                    }
                });
                err?;
                HandlerStatus::Handled
            }
            ext::Base::G(cmd) => {
                let mut regs: <T::Arch as Arch>::Registers = Default::default();
                regs.gdb_deserialize(cmd.vals)
                    .map_err(|_| Error::TargetMismatch)?;

                match target.base_ops() {
                    BaseOps::SingleThread(ops) => ops.write_registers(&regs),
                    BaseOps::MultiThread(ops) => ops.write_registers(&regs, self.current_mem_tid),
                }
                .handle_error()?;

                HandlerStatus::NeedsOK
            }
            ext::Base::m(cmd) => {
                let buf = cmd.buf;
                let addr = <T::Arch as Arch>::Usize::from_be_bytes(cmd.addr)
                    .ok_or(Error::TargetMismatch)?;

                let mut i = 0;
                let mut n = cmd.len;
                while n != 0 {
                    let chunk_size = n.min(buf.len());

                    use num_traits::NumCast;

                    let addr = addr + NumCast::from(i).ok_or(Error::TargetMismatch)?;
                    let data = &mut buf[..chunk_size];
                    match target.base_ops() {
                        BaseOps::SingleThread(ops) => ops.read_addrs(addr, data),
                        BaseOps::MultiThread(ops) => {
                            ops.read_addrs(addr, data, self.current_mem_tid)
                        }
                    }
                    .handle_error()?;

                    n -= chunk_size;
                    i += chunk_size;

                    res.write_hex_buf(data)?;
                }
                HandlerStatus::Handled
            }
            ext::Base::M(cmd) => {
                let addr = <T::Arch as Arch>::Usize::from_be_bytes(cmd.addr)
                    .ok_or(Error::TargetMismatch)?;

                match target.base_ops() {
                    BaseOps::SingleThread(ops) => ops.write_addrs(addr, cmd.val),
                    BaseOps::MultiThread(ops) => {
                        ops.write_addrs(addr, cmd.val, self.current_mem_tid)
                    }
                }
                .handle_error()?;

                HandlerStatus::NeedsOK
            }
            ext::Base::k(_) | ext::Base::vKill(_) => {
                match target.extended_mode() {
                    // When not running in extended mode, stop the `GdbStub` and disconnect.
                    None => HandlerStatus::Disconnect(DisconnectReason::Kill),

                    // When running in extended mode, a kill command does not necessarily result in
                    // a disconnect...
                    Some(ops) => {
                        let pid = match command {
                            ext::Base::vKill(cmd) => Some(cmd.pid),
                            _ => None,
                        };

                        let should_terminate = ops.kill(pid).handle_error()?;
                        if should_terminate.into() {
                            // manually write OK, since we need to return a DisconnectReason
                            res.write_str("OK")?;
                            HandlerStatus::Disconnect(DisconnectReason::Kill)
                        } else {
                            HandlerStatus::NeedsOK
                        }
                    }
                }
            }
            ext::Base::D(_) => {
                // TODO: plumb-through Pid when exposing full multiprocess + extended mode
                res.write_str("OK")?; // manually write OK, since we need to return a DisconnectReason
                HandlerStatus::Disconnect(DisconnectReason::Disconnect)
            }
            ext::Base::Z(cmd) => {
                let addr = <T::Arch as Arch>::Usize::from_be_bytes(cmd.addr)
                    .ok_or(Error::TargetMismatch)?;

                use crate::target::ext::breakpoints::WatchKind::*;
                let supported = match cmd.type_ {
                    0 => (target.sw_breakpoint()).map(|op| op.add_sw_breakpoint(addr)),
                    1 => (target.hw_breakpoint()).map(|op| op.add_hw_breakpoint(addr)),
                    2 => (target.hw_watchpoint()).map(|op| op.add_hw_watchpoint(addr, Write)),
                    3 => (target.hw_watchpoint()).map(|op| op.add_hw_watchpoint(addr, Read)),
                    4 => (target.hw_watchpoint()).map(|op| op.add_hw_watchpoint(addr, ReadWrite)),
                    // only 5 types in the protocol
                    _ => None,
                };

                match supported {
                    None => HandlerStatus::Handled,
                    Some(Err(e)) => {
                        Err(e).handle_error()?;
                        HandlerStatus::Handled
                    }
                    Some(Ok(true)) => HandlerStatus::NeedsOK,
                    Some(Ok(false)) => return Err(Error::NonFatalError(22)),
                }
            }
            ext::Base::z(cmd) => {
                let addr = <T::Arch as Arch>::Usize::from_be_bytes(cmd.addr)
                    .ok_or(Error::TargetMismatch)?;

                use crate::target::ext::breakpoints::WatchKind::*;
                let supported = match cmd.type_ {
                    0 => (target.sw_breakpoint()).map(|op| op.remove_sw_breakpoint(addr)),
                    1 => (target.hw_breakpoint()).map(|op| op.remove_hw_breakpoint(addr)),
                    2 => (target.hw_watchpoint()).map(|op| op.remove_hw_watchpoint(addr, Write)),
                    3 => (target.hw_watchpoint()).map(|op| op.remove_hw_watchpoint(addr, Read)),
                    4 => {
                        (target.hw_watchpoint()).map(|op| op.remove_hw_watchpoint(addr, ReadWrite))
                    }
                    // only 5 types in the protocol
                    _ => None,
                };

                match supported {
                    None => HandlerStatus::Handled,
                    Some(Err(e)) => {
                        Err(e).handle_error()?;
                        HandlerStatus::Handled
                    }
                    Some(Ok(true)) => HandlerStatus::NeedsOK,
                    Some(Ok(false)) => return Err(Error::NonFatalError(22)),
                }
            }
            ext::Base::p(p) => {
                let mut dst = [0u8; 32]; // enough for 256-bit registers
                let reg = <T::Arch as Arch>::RegId::from_raw_id(p.reg_id);
                let (reg_id, reg_size) = match reg {
                    Some(v) => v,
                    // empty packet indicates unrecognized query
                    None => return Ok(HandlerStatus::Handled),
                };
                let dst = &mut dst[0..reg_size];
                match target.base_ops() {
                    BaseOps::SingleThread(ops) => ops.read_register(reg_id, dst),
                    BaseOps::MultiThread(ops) => {
                        ops.read_register(reg_id, dst, self.current_mem_tid)
                    }
                }
                .handle_error()?;

                res.write_hex_buf(dst)?;
                HandlerStatus::Handled
            }
            ext::Base::P(p) => {
                let reg = <T::Arch as Arch>::RegId::from_raw_id(p.reg_id);
                match reg {
                    None => return Err(Error::NonFatalError(22)),
                    Some((reg_id, _)) => match target.base_ops() {
                        BaseOps::SingleThread(ops) => ops.write_register(reg_id, p.val),
                        BaseOps::MultiThread(ops) => {
                            ops.write_register(reg_id, p.val, self.current_mem_tid)
                        }
                    }
                    .handle_error()?,
                }
                HandlerStatus::NeedsOK
            }
            ext::Base::vCont(cmd) => {
                use crate::protocol::commands::_vCont::{vCont, VContKind};

                let actions = match cmd {
                    vCont::Query => {
                        res.write_str("vCont;c;C;s;S")?;
                        return Ok(HandlerStatus::Handled);
                    }
                    vCont::Actions(actions) => actions,
                };

                // map raw vCont action iterator to a format the `Target` expects
                let mut err = Ok(());
                let mut actions = actions.into_iter().filter_map(|action| {
                    let action = match action {
                        Some(action) => action,
                        None => {
                            err = Err(Error::PacketParse(
                                crate::protocol::PacketParseError::MalformedCommand,
                            ));
                            return None;
                        }
                    };

                    let resume_action = match action.kind {
                        VContKind::Step => ResumeAction::Step,
                        VContKind::Continue => ResumeAction::Continue,
                        _ => {
                            // there seems to be a GDB bug where it doesn't use `vCont` unless
                            // `vCont?` returns support for resuming with a signal.
                            //
                            // This error case can be removed once "Resume with Signal" is
                            // implemented
                            err = Err(Error::ResumeWithSignalUnimplemented);
                            return None;
                        }
                    };

                    let tid = match action.thread {
                        Some(thread) => match thread.tid {
                            IdKind::Any => {
                                err = Err(Error::PacketUnexpected);
                                return None;
                            }
                            IdKind::All => TidSelector::All,
                            IdKind::WithID(tid) => TidSelector::WithID(tid),
                        },
                        // An action with no thread-id matches all threads
                        None => TidSelector::All,
                    };

                    Some((tid, resume_action))
                });

                let ret = match self.do_vcont(res, target, &mut actions) {
                    Ok(None) => HandlerStatus::Handled,
                    Ok(Some(dc)) => HandlerStatus::Disconnect(dc),
                    Err(e) => return Err(e),
                };
                err?;
                ret
            }
            // TODO?: support custom resume addr in 'c' and 's'
            ext::Base::c(_) => {
                match self.do_vcont(
                    res,
                    target,
                    &mut core::iter::once((self.current_resume_tid, ResumeAction::Continue)),
                ) {
                    Ok(None) => HandlerStatus::Handled,
                    Ok(Some(dc)) => HandlerStatus::Disconnect(dc),
                    Err(e) => return Err(e),
                }
            }
            ext::Base::s(_) => {
                match self.do_vcont(
                    res,
                    target,
                    &mut core::iter::once((self.current_resume_tid, ResumeAction::Step)),
                ) {
                    Ok(None) => HandlerStatus::Handled,
                    Ok(Some(dc)) => HandlerStatus::Disconnect(dc),
                    Err(e) => return Err(e),
                }
            }

            // ------------------- Multi-threading Support ------------------ //
            ext::Base::H(cmd) => {
                use crate::protocol::commands::_h_upcase::Op;
                match cmd.kind {
                    Op::Other => match cmd.thread.tid {
                        IdKind::Any => {} // reuse old tid
                        // "All" threads doesn't make sense for memory accesses
                        IdKind::All => return Err(Error::PacketUnexpected),
                        IdKind::WithID(tid) => self.current_mem_tid = tid,
                    },
                    // technically, this variant is deprecated in favor of vCont...
                    Op::StepContinue => match cmd.thread.tid {
                        IdKind::Any => {} // reuse old tid
                        IdKind::All => self.current_resume_tid = TidSelector::All,
                        IdKind::WithID(tid) => self.current_resume_tid = TidSelector::WithID(tid),
                    },
                }
                HandlerStatus::NeedsOK
            }
            ext::Base::qfThreadInfo(_) => {
                res.write_str("m")?;

                match target.base_ops() {
                    BaseOps::SingleThread(_) => res.write_thread_id(ThreadId {
                        pid: Some(IdKind::WithID(FAKE_PID)),
                        tid: IdKind::WithID(SINGLE_THREAD_TID),
                    })?,
                    BaseOps::MultiThread(ops) => {
                        let mut err: Result<_, Error<T::Error, C::Error>> = Ok(());
                        let mut first = true;
                        ops.list_active_threads(&mut |tid| {
                            // TODO: replace this with a try block (once stabilized)
                            let e = (|| {
                                if !first {
                                    res.write_str(",")?
                                }
                                first = false;
                                res.write_thread_id(ThreadId {
                                    pid: Some(IdKind::WithID(FAKE_PID)),
                                    tid: IdKind::WithID(tid),
                                })?;
                                Ok(())
                            })();

                            if let Err(e) = e {
                                err = Err(e)
                            }
                        })
                        .map_err(Error::TargetError)?;
                        err?;
                    }
                }

                HandlerStatus::Handled
            }
            ext::Base::qsThreadInfo(_) => {
                res.write_str("l")?;
                HandlerStatus::Handled
            }
            ext::Base::T(cmd) => {
                let alive = match cmd.thread.tid {
                    IdKind::WithID(tid) => match target.base_ops() {
                        BaseOps::SingleThread(_) => tid == SINGLE_THREAD_TID,
                        BaseOps::MultiThread(ops) => {
                            ops.is_thread_alive(tid).map_err(Error::TargetError)?
                        }
                    },
                    // TODO: double-check if GDB ever sends other variants
                    // Even after ample testing, this arm has never been hit...
                    _ => return Err(Error::PacketUnexpected),
                };
                if alive {
                    HandlerStatus::NeedsOK
                } else {
                    // any error code will do
                    return Err(Error::NonFatalError(1));
                }
            }
        };
        Ok(handler_status)
    }

    fn handle_monitor_cmd<'a>(
        &mut self,
        res: &mut ResponseWriter<C>,
        target: &mut T,
        command: ext::MonitorCmd<'a>,
    ) -> Result<HandlerStatus, Error<T::Error, C::Error>> {
        let ops = match target.monitor_cmd() {
            Some(ops) => ops,
            None => return Ok(HandlerStatus::Handled),
        };

        let handler_status = match command {
            ext::MonitorCmd::qRcmd(cmd) => {
                crate::__dead_code_marker!("qRcmd", "impl");

                let mut err: Result<_, Error<T::Error, C::Error>> = Ok(());
                let mut callback = |msg: &[u8]| {
                    // TODO: replace this with a try block (once stabilized)
                    let e = (|| {
                        let mut res = ResponseWriter::new(res.as_conn());
                        res.write_str("O")?;
                        res.write_hex_buf(msg)?;
                        res.flush()?;
                        Ok(())
                    })();

                    if let Err(e) = e {
                        err = Err(e)
                    }
                };

                ops.handle_monitor_cmd(cmd.hex_cmd, ConsoleOutput::new(&mut callback))
                    .map_err(Error::TargetError)?;
                err?;

                HandlerStatus::NeedsOK
            }
        };

        Ok(handler_status)
    }

    fn handle_section_offsets(
        &mut self,
        res: &mut ResponseWriter<C>,
        target: &mut T,
        command: ext::SectionOffsets,
    ) -> Result<HandlerStatus, Error<T::Error, C::Error>> {
        let ops = match target.section_offsets() {
            Some(ops) => ops,
            None => return Ok(HandlerStatus::Handled),
        };

        let handler_status = match command {
            ext::SectionOffsets::qOffsets(_cmd) => {
                use crate::target::ext::section_offsets::Offsets;

                crate::__dead_code_marker!("qOffsets", "impl");

                match ops.get_section_offsets().map_err(Error::TargetError)? {
                    Offsets::Sections { text, data, bss } => {
                        res.write_str("Text=")?;
                        res.write_num(text)?;

                        res.write_str(";Data=")?;
                        res.write_num(data)?;

                        // "Note: while a Bss offset may be included in the response,
                        // GDB ignores this and instead applies the Data offset to the Bss section."
                        //
                        // While this would suggest that it's OK to omit `Bss=` entirely, recent
                        // versions of GDB seem to require that `Bss=` is present.
                        //
                        // See https://github.com/bminor/binutils-gdb/blob/master/gdb/remote.c#L4149-L4159
                        let bss = bss.unwrap_or(data);
                        res.write_str(";Bss=")?;
                        res.write_num(bss)?;
                    }
                    Offsets::Segments { text_seg, data_seg } => {
                        res.write_str("TextSeg=")?;
                        res.write_num(text_seg)?;

                        if let Some(data) = data_seg {
                            res.write_str(";DataSeg=")?;
                            res.write_num(data)?;
                        }
                    }
                }
                HandlerStatus::Handled
            }
        };

        Ok(handler_status)
    }

    fn handle_extended_mode<'a>(
        &mut self,
        res: &mut ResponseWriter<C>,
        target: &mut T,
        command: ext::ExtendedMode<'a>,
    ) -> Result<HandlerStatus, Error<T::Error, C::Error>> {
        let ops = match target.extended_mode() {
            Some(ops) => ops,
            None => return Ok(HandlerStatus::Handled),
        };

        let handler_status = match command {
            ext::ExtendedMode::ExclamationMark(_cmd) => {
                ops.on_start().map_err(Error::TargetError)?;
                HandlerStatus::NeedsOK
            }
            ext::ExtendedMode::R(_cmd) => {
                ops.restart().map_err(Error::TargetError)?;
                HandlerStatus::Handled
            }
            ext::ExtendedMode::vAttach(cmd) => {
                ops.attach(cmd.pid).handle_error()?;

                #[cfg(feature = "alloc")]
                self.attached_pids.insert(cmd.pid, true);

                // TODO: sends OK when running in Non-Stop mode
                HandlerStatus::Handled
            }
            ext::ExtendedMode::vRun(cmd) => {
                use crate::target::ext::extended_mode::Args;

                let mut pid = ops
                    .run(cmd.filename, Args::new(&mut cmd.args.into_iter()))
                    .handle_error()?;

                // on single-threaded systems, we'll ignore the provided PID and keep
                // using the FAKE_PID.
                if let BaseOps::SingleThread(_) = target.base_ops() {
                    pid = FAKE_PID;
                }

                let _ = pid; // squelch warning on no_std targets
                #[cfg(feature = "alloc")]
                self.attached_pids.insert(pid, false);

                // TODO: send a more descriptive stop packet?
                res.write_str("S05")?;
                HandlerStatus::Handled
            }
            // --------- ASLR --------- //
            ext::ExtendedMode::QDisableRandomization(cmd) if ops.configure_aslr().is_some() => {
                let ops = ops.configure_aslr().unwrap();
                ops.cfg_aslr(cmd.value).handle_error()?;
                HandlerStatus::NeedsOK
            }
            // --------- Environment --------- //
            ext::ExtendedMode::QEnvironmentHexEncoded(cmd) if ops.configure_env().is_some() => {
                let ops = ops.configure_env().unwrap();
                ops.set_env(cmd.key, cmd.value).handle_error()?;
                HandlerStatus::NeedsOK
            }
            ext::ExtendedMode::QEnvironmentUnset(cmd) if ops.configure_env().is_some() => {
                let ops = ops.configure_env().unwrap();
                ops.remove_env(cmd.key).handle_error()?;
                HandlerStatus::NeedsOK
            }
            ext::ExtendedMode::QEnvironmentReset(_cmd) if ops.configure_env().is_some() => {
                let ops = ops.configure_env().unwrap();
                ops.reset_env().handle_error()?;
                HandlerStatus::NeedsOK
            }
            // --------- Working Dir --------- //
            ext::ExtendedMode::QSetWorkingDir(cmd) if ops.configure_working_dir().is_some() => {
                let ops = ops.configure_working_dir().unwrap();
                ops.cfg_working_dir(cmd.dir).handle_error()?;
                HandlerStatus::NeedsOK
            }
            // --------- Startup Shell --------- //
            ext::ExtendedMode::QStartupWithShell(cmd)
                if ops.configure_startup_shell().is_some() =>
            {
                let ops = ops.configure_startup_shell().unwrap();
                ops.cfg_startup_with_shell(cmd.value).handle_error()?;
                HandlerStatus::NeedsOK
            }
            _ => HandlerStatus::Handled,
        };

        Ok(handler_status)
    }

    fn do_vcont(
        &mut self,
        res: &mut ResponseWriter<C>,
        target: &mut T,
        actions: &mut dyn Iterator<Item = (TidSelector, ResumeAction)>,
    ) -> Result<Option<DisconnectReason>, Error<T::Error, C::Error>> {
        let mut err = Ok(());

        let mut check_gdb_interrupt = || match res.as_conn().peek() {
            Ok(Some(0x03)) => true, // 0x03 is the interrupt byte
            Ok(Some(_)) => false,   // it's nothing that can't wait...
            Ok(None) => false,
            Err(e) => {
                err = Err(Error::ConnectionRead(e));
                true // break ASAP if a connection error occurred
            }
        };

        let stop_reason = match target.base_ops() {
            BaseOps::SingleThread(ops) => ops
                .resume(
                    // TODO?: add a more descriptive error if vcont has multiple threads in
                    // single-threaded mode?
                    actions.next().ok_or(Error::PacketUnexpected)?.1,
                    &mut check_gdb_interrupt,
                )
                .map_err(Error::TargetError)?
                .into(),
            BaseOps::MultiThread(ops) => ops
                .resume(Actions::new(actions), &mut check_gdb_interrupt)
                .map_err(Error::TargetError)?,
        };

        err?;

        self.finish_vcont(stop_reason, res)
    }

    // DEVNOTE: `do_vcont` and `finish_vcont` could be merged into a single
    // function, at the expense of slightly larger code. In the future, if the
    // `vCont` machinery is re-written, there's no reason why the two functions
    // couldn't be re-merged.

    fn finish_vcont(
        &mut self,
        stop_reason: ThreadStopReason<<T::Arch as Arch>::Usize>,
        res: &mut ResponseWriter<C>,
    ) -> Result<Option<DisconnectReason>, Error<T::Error, C::Error>> {
        match stop_reason {
            ThreadStopReason::DoneStep | ThreadStopReason::GdbInterrupt => {
                res.write_str("S05")?;
                Ok(None)
            }
            ThreadStopReason::Signal(code) => {
                res.write_str("S")?;
                res.write_num(code)?;
                Ok(None)
            }
            ThreadStopReason::Halted => {
                res.write_str("W19")?; // SIGSTOP
                Ok(Some(DisconnectReason::TargetHalted))
            }
            ThreadStopReason::SwBreak(tid)
            | ThreadStopReason::HwBreak(tid)
            | ThreadStopReason::Watch { tid, .. } => {
                self.current_mem_tid = tid;
                self.current_resume_tid = TidSelector::WithID(tid);

                res.write_str("T05")?;

                res.write_str("thread:")?;
                res.write_thread_id(ThreadId {
                    pid: Some(IdKind::WithID(FAKE_PID)),
                    tid: IdKind::WithID(tid),
                })?;
                res.write_str(";")?;

                match stop_reason {
                    // don't include addr on sw/hw break
                    ThreadStopReason::SwBreak(_) => res.write_str("swbreak:")?,
                    ThreadStopReason::HwBreak(_) => res.write_str("hwbreak:")?,
                    ThreadStopReason::Watch { kind, addr, .. } => {
                        use crate::target::ext::breakpoints::WatchKind;
                        match kind {
                            WatchKind::Write => res.write_str("watch:")?,
                            WatchKind::Read => res.write_str("rwatch:")?,
                            WatchKind::ReadWrite => res.write_str("awatch:")?,
                        }
                        res.write_num(addr)?;
                    }
                    _ => unreachable!(),
                };

                res.write_str(";")?;
                Ok(None)
            }
        }
    }
}

use crate::target::ext::base::singlethread::StopReason;
impl<U> From<StopReason<U>> for ThreadStopReason<U> {
    fn from(st_stop_reason: StopReason<U>) -> ThreadStopReason<U> {
        match st_stop_reason {
            StopReason::DoneStep => ThreadStopReason::DoneStep,
            StopReason::GdbInterrupt => ThreadStopReason::GdbInterrupt,
            StopReason::Halted => ThreadStopReason::Halted,
            StopReason::SwBreak => ThreadStopReason::SwBreak(SINGLE_THREAD_TID),
            StopReason::HwBreak => ThreadStopReason::HwBreak(SINGLE_THREAD_TID),
            StopReason::Watch { kind, addr } => ThreadStopReason::Watch {
                tid: SINGLE_THREAD_TID,
                kind,
                addr,
            },
            StopReason::Signal(sig) => ThreadStopReason::Signal(sig),
        }
    }
}
