use core::marker::PhantomData;

use managed::ManagedSlice;

use crate::protocol::commands::ParseCommand;
use crate::{
    arch::{Arch, RegId, Registers},
    connection::Connection,
    internal::*,
    protocol::{Command, ConsoleOutput, IdKind, Packet, ResponseWriter, ThreadId},
    target::base::multithread::{Actions, ResumeAction, ThreadStopReason, TidSelector},
    target::base::BaseOps,
    target::{ext, Target},
    util::managed_vec::ManagedVec,
    Tid, FAKE_PID, SINGLE_THREAD_TID,
};

mod builder;
mod error;

pub use builder::{GdbStubBuilder, GdbStubBuilderError};
pub use error::GdbStubError;

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

/// Debug a [`Target`](trait.Target.html) across a
/// [`Connection`](trait.Connection.html) using the GDB Remote Serial Protocol.
pub struct GdbStub<'a, T: Target, C: Connection> {
    conn: C,
    packet_buffer: ManagedSlice<'a, u8>,
    state: GdbStubImpl<T, C>,
}

impl<'a, T: Target, C: Connection> GdbStub<'a, T, C> {
    /// Create a `GdbStubBuilder` using the provided Connection.
    pub fn builder(conn: C) -> GdbStubBuilder<'a, T, C> {
        GdbStubBuilder::new(conn)
    }

    /// Create a new `GdbStub` using the provided connection.
    ///
    /// For fine-grained control over various GdbStub options, use the
    /// [`builder()`](#method.builder) method instead.
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
    pub fn run<'b>(
        &mut self,
        target: &'b mut T,
    ) -> Result<DisconnectReason, Error<T::Error, C::Error>> {
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
                        Ok(reason) => reason,
                        Err(Error::TargetError(e)) => {
                            // unlike all other errors, which are "unrecoverable", there's a chance
                            // that a target may be able to recover from a target-specific error. In
                            // this case, we may as well report a SIGABRT stop reason, giving the
                            // target a chance to open a "post-mortem" GDB session.
                            let mut res = ResponseWriter::new(conn);
                            res.write_str("T06")?; // SIGABRT
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
        command: Command<'_>,
    ) -> Result<Option<DisconnectReason>, Error<T::Error, C::Error>> {
        match command {
            // ------------------ Handshaking and Queries ------------------- //
            Command::qSupported(cmd) => {
                // XXX: actually read what the client supports, and enable/disable features
                // appropriately
                let _features = cmd.features.into_iter();

                res.write_str("PacketSize=")?;
                res.write_num(self.packet_buffer_len)?;

                res.write_str(";vContSupported+")?;
                res.write_str(";multiprocess+")?;
                res.write_str(";QStartNoAckMode+")?;

                res.write_str(";swbreak+")?;
                if target.hw_breakpoint().is_some() || target.hw_watchpoint().is_some() {
                    res.write_str(";hwbreak+")?;
                }

                // TODO: implement conditional breakpoint support (since that's kool).
                // res.write_str("ConditionalBreakpoints+;")?;

                if T::Arch::target_description_xml().is_some() {
                    res.write_str(";qXfer:features:read+")?;
                }
            }
            Command::QStartNoAckMode(_) => {
                self.no_ack_mode = true;
                res.write_str("OK")?;
            }
            Command::vContQuestionMark(_) => res.write_str("vCont;c;C;s;S")?,
            Command::qXferFeaturesRead(cmd) => {
                assert_eq!(cmd.annex, "target.xml");
                match T::Arch::target_description_xml() {
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
            }

            // -------------------- "Core" Functionality -------------------- //
            // TODO: Improve the '?' response based on last-sent stop reason.
            Command::QuestionMark(_) => res.write_str("S05")?,
            Command::qAttached(_) => res.write_str("1")?, // attached to existing process
            Command::g(_) => {
                let mut regs: <T::Arch as Arch>::Registers = Default::default();
                match target.base_ops() {
                    BaseOps::SingleThread(ops) => ops.read_registers(&mut regs),
                    BaseOps::MultiThread(ops) => {
                        ops.read_registers(&mut regs, self.current_mem_tid)
                    }
                }
                .map_err(Error::TargetError)?;

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
            }
            Command::G(cmd) => {
                let mut regs: <T::Arch as Arch>::Registers = Default::default();
                regs.gdb_deserialize(cmd.vals)
                    .map_err(|_| Error::TargetMismatch)?;

                match target.base_ops() {
                    BaseOps::SingleThread(ops) => ops.write_registers(&regs),
                    BaseOps::MultiThread(ops) => ops.write_registers(&regs, self.current_mem_tid),
                }
                .map_err(Error::TargetError)?;

                res.write_str("OK")?;
            }
            Command::m(cmd) => {
                let buf = cmd.buf;

                let mut i = 0;
                let mut n = cmd.len;
                while n != 0 {
                    let chunk_size = n.min(buf.len());

                    let addr = Self::to_target_usize(cmd.addr + i as u64)?;
                    let data = &mut buf[..chunk_size];
                    let success = match target.base_ops() {
                        BaseOps::SingleThread(ops) => ops.read_addrs(addr, data),
                        BaseOps::MultiThread(ops) => {
                            ops.read_addrs(addr, data, self.current_mem_tid)
                        }
                    }
                    .map_err(Error::TargetError)?;

                    if !success {
                        debug!("invalid memory read!");
                        break;
                    }

                    n -= chunk_size;
                    i += chunk_size;

                    res.write_hex_buf(data)?;
                }
            }
            Command::M(cmd) => {
                let addr = Self::to_target_usize(cmd.addr)?;

                let success = match target.base_ops() {
                    BaseOps::SingleThread(ops) => ops.write_addrs(addr, cmd.val),
                    BaseOps::MultiThread(ops) => {
                        ops.write_addrs(addr, cmd.val, self.current_mem_tid)
                    }
                }
                .map_err(Error::TargetError)?;

                if !success {
                    res.write_str("E14")? // error code grafted from QEMU
                } else {
                    res.write_str("OK")?;
                }
            }
            Command::k(_) | Command::vKill(_) => {
                // no response
                return Ok(Some(DisconnectReason::Kill));
            }
            Command::D(_) => {
                res.write_str("OK")?;
                return Ok(Some(DisconnectReason::Disconnect));
            }
            Command::Z(cmd) => {
                let addr = Self::to_target_usize(cmd.addr)?;

                use ext::breakpoint::WatchKind::*;
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
                    None => {}
                    Some(Err(e)) => return Err(Error::TargetError(e)),
                    Some(Ok(true)) => res.write_str("OK")?,
                    Some(Ok(false)) => res.write_str("E22")?, // value of 22 grafted from QEMU
                }
            }
            Command::z(cmd) => {
                let addr = Self::to_target_usize(cmd.addr)?;

                use ext::breakpoint::WatchKind::*;
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
                    None => {}
                    Some(Err(e)) => return Err(Error::TargetError(e)),
                    Some(Ok(true)) => res.write_str("OK")?,
                    Some(Ok(false)) => res.write_str("E22")?, // value of 22 grafted from QEMU
                }
            }
            Command::p(p) => {
                let mut dst = [0u8; 32]; // enough for 256-bit registers
                let reg = <T::Arch as Arch>::RegId::from_raw_id(p.reg_id);
                let (reg_id, reg_size) = match reg {
                    Some(v) => v,
                    None => return Ok(None),
                };
                let dst = &mut dst[0..reg_size];
                let supported = match target.base_ops() {
                    BaseOps::SingleThread(ops) => {
                        ops.read_register(reg_id, dst).map_err(Error::TargetError)?
                    }
                    BaseOps::MultiThread(ops) => ops
                        .read_register(reg_id, dst, self.current_mem_tid)
                        .map_err(Error::TargetError)?,
                };
                if supported {
                    res.write_hex_buf(dst)?;
                } else {
                    res.write_str("E01")?;
                }
            }
            Command::P(p) => {
                let reg = <T::Arch as Arch>::RegId::from_raw_id(p.reg_id);
                let supported = match reg {
                    Some((reg_id, _)) => match target.base_ops() {
                        BaseOps::SingleThread(ops) => ops
                            .write_register(reg_id, p.val)
                            .map_err(Error::TargetError)?,
                        BaseOps::MultiThread(ops) => ops
                            .write_register(reg_id, p.val, self.current_mem_tid)
                            .map_err(Error::TargetError)?,
                    },
                    None => false,
                };
                if supported {
                    res.write_str("OK")?;
                } else {
                    res.write_str("E01")?;
                }
            }
            Command::vCont(cmd) => {
                use crate::protocol::commands::_vCont::VContKind;

                // map raw vCont action iterator to a format the `Target` expects
                let mut err = Ok(());
                let mut actions = cmd.actions.into_iter().filter_map(|action| {
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

                let ret = self.do_vcont(res, target, &mut actions);
                err?;
                return ret;
            }
            // TODO?: support custom resume addr in 'c' and 's'
            Command::c(_) => {
                return self.do_vcont(
                    res,
                    target,
                    &mut core::iter::once((self.current_resume_tid, ResumeAction::Continue)),
                )
            }
            Command::s(_) => {
                return self.do_vcont(
                    res,
                    target,
                    &mut core::iter::once((self.current_resume_tid, ResumeAction::Step)),
                )
            }

            // ------------------- Multi-threading Support ------------------ //
            Command::H(cmd) => {
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
                res.write_str("OK")?
            }
            Command::qfThreadInfo(_) => {
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
            }
            Command::qsThreadInfo(_) => res.write_str("l")?,
            Command::T(cmd) => {
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
                    res.write_str("OK")?;
                } else {
                    res.write_str("E01")?; // TODO: is this an okay error code?
                }
            }

            // -------------------- Protocol Extensions --------------------- //
            // > Hey, what's with the calls to `__protocol_hint_(target)`?
            //
            // If a target doesn't implement the packet's corresponding protocol extension, it would
            // be nice if the Rust compiler would simply "omit" the entire packet handler from the
            // output binary. The `__protocol_hint_` method "nudges" the Rust compiler in the right
            // direction, and basically guarantees that the match arm is optimized out in release
            // build binaries.
            //
            // Of course, this optimization likely won't occur if `GdbStub` is monomorphised with a
            // `Box<Target>` or `&dyn Target`. Fortunately, in those cases, it's likely that
            // `gdbstub` is being used in a "hosted" environment (e.g: an emulator), where a bit
            // more binary bloat won't necessarily be the end of the world...
            Command::qRcmd(cmd) if cmd.__protocol_hint_(target) => {
                crate::__dead_code_marker!("qRcmd", "impl");

                if let Some(ops) = target.monitor_cmd() {
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

                    res.write_str("OK")?
                }
            }

            Command::qOffsets(cmd) if cmd.__protocol_hint_(target) => {
                use ext::section_offsets::Offsets;

                crate::__dead_code_marker!("qOffsets", "impl");

                if let Some(op) = target.section_offsets() {
                    match op.get_section_offsets().map_err(Error::TargetError)? {
                        Offsets::Sections { text, data, bss } => {
                            res.write_str("Text=")?;
                            res.write_num(text)?;

                            res.write_str(";Data=")?;
                            res.write_num(data)?;

                            if let Some(data) = bss {
                                res.write_str(";Bss=")?;
                                res.write_num(data)?;
                            }
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
                }
            }

            // -------------------------------------------------------------- //
            Command::Unknown(cmd) => info!("Unknown command: {}", cmd),
            _ => trace!("Unimplemented command"),
        }

        Ok(None)
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
                        use ext::breakpoint::WatchKind;
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

    #[allow(clippy::wrong_self_convention, clippy::type_complexity)]
    fn to_target_usize(
        n: impl BeBytes,
    ) -> Result<<T::Arch as Arch>::Usize, Error<T::Error, C::Error>> {
        let mut buf = [0; 16]; // enough for 128-bit pointers
        let len = n.to_be_bytes(&mut buf).ok_or(Error::TargetMismatch)?;
        <T::Arch as Arch>::Usize::from_be_bytes(&buf[..len]).ok_or(Error::TargetMismatch)
    }
}

use crate::target::base::singlethread::StopReason;
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
