use core::marker::PhantomData;

use managed::ManagedSlice;

use crate::{
    arch::{Arch, Registers},
    connection::Connection,
    internal::*,
    protocol::{Command, ConsoleOutput, Packet, ResponseWriter, Tid, TidSelector},
    target::{Actions, BreakOp, ResumeAction, StopReason, Target, WatchKind},
    util::managed_vec::ManagedVec,
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
    current_tid: Tid,
    multithread: bool,
}

impl<T: Target, C: Connection> GdbStubImpl<T, C> {
    fn new(packet_buffer_len: usize) -> GdbStubImpl<T, C> {
        GdbStubImpl {
            _target: PhantomData,
            _connection: PhantomData,

            packet_buffer_len,
            current_tid: Tid {
                pid: None,
                tid: TidSelector::Any,
            },
            multithread: false,
        }
    }

    fn run(
        &mut self,
        target: &mut T,
        conn: &mut C,
        packet_buffer: &mut ManagedSlice<u8>,
    ) -> Result<DisconnectReason, Error<T::Error, C::Error>> {
        loop {
            match Self::recv_packet(conn, packet_buffer)? {
                Packet::Ack => {}
                Packet::Nack => {
                    unimplemented!("GDB nack'd the packet, but retry isn't implemented yet")
                }
                Packet::Interrupt => {
                    debug!("<-- interrupt packet");
                    let mut res = ResponseWriter::new(conn);
                    res.write_str("S05")?;
                    res.flush()?;
                }
                Packet::Command(command) => {
                    // Acknowledge the command
                    conn.write(b'+').map_err(Error::ConnectionRead)?;

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

    fn recv_packet<'a, 'b>(
        conn: &mut C,
        pkt_buf: &'a mut ManagedSlice<'b, u8>,
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

        drop(buf);

        let len = pkt_buf.len();
        match Packet::from_buf(&mut pkt_buf.as_mut()[..len]) {
            Ok(packet) => Ok(packet),
            Err(e) => {
                // TODO: preserve this context within Error::PacketParse
                error!("Could not parse packet: {:?}", e);
                Err(Error::PacketParse)
            }
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
                res.write_str(";swbreak+")?;

                // probe support for various watchpoints/breakpoints
                let mut supports_hwbreak = false;

                let test_addr = num_traits::zero();
                if target
                    .update_hw_breakpoint(test_addr, BreakOp::Add)
                    .maybe_missing_impl()?
                    .is_some()
                {
                    // FIXME: properly assert this succeeded
                    let _ = target.update_hw_breakpoint(test_addr, BreakOp::Remove);
                    supports_hwbreak = true;
                }
                if (target.update_hw_watchpoint(test_addr, BreakOp::Add, WatchKind::Write))
                    .maybe_missing_impl()?
                    .is_some()
                {
                    // FIXME: properly assert this succeeded
                    let _ =
                        target.update_hw_watchpoint(test_addr, BreakOp::Remove, WatchKind::Write);
                    supports_hwbreak = true;
                }

                if supports_hwbreak {
                    res.write_str(";hwbreak+")?;
                }

                // TODO: implement conditional breakpoint support (since that's kool).
                // res.write_str("ConditionalBreakpoints+;")?;

                // probe support for target description xml
                if T::Arch::target_description_xml().is_some() {
                    res.write_str(";qXfer:features:read+")?;
                }
            }
            Command::vContQuestionMark(_) => res.write_str("vCont;c;s")?,
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
                // sanity check that the target has implemented `set_current_thread` when
                // running in multithreaded mode.
                if self.multithread {
                    if let TidSelector::WithID(tid) = self.current_tid.tid {
                        target
                            .set_current_thread(tid)
                            .maybe_missing_impl()?
                            .ok_or(Error::MissingSetCurrentTid)?
                    }
                }

                let mut regs: <T::Arch as Arch>::Registers = Default::default();
                target
                    .read_registers(&mut regs)
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
                // sanity check that the target has implemented `set_current_thread` when
                // running in multithreaded mode.
                if self.multithread {
                    if let TidSelector::WithID(tid) = self.current_tid.tid {
                        target
                            .set_current_thread(tid)
                            .maybe_missing_impl()?
                            .ok_or(Error::MissingSetCurrentTid)?
                    }
                }

                let mut regs: <T::Arch as Arch>::Registers = Default::default();
                regs.gdb_deserialize(cmd.vals)
                    .map_err(|_| Error::PacketParse)?; // FIXME: more granular error?
                target.write_registers(&regs).map_err(Error::TargetError)?;
                res.write_str("OK")?;
            }
            Command::m(cmd) => {
                let buf = cmd.buf;

                let mut i = 0;
                let mut n = cmd.len;
                while n != 0 {
                    let chunk_size = n.min(buf.len());

                    let start_addr = Self::to_target_usize(cmd.addr + i as u64)?;
                    let data = &mut buf[..chunk_size];
                    let success = target
                        .read_addrs(start_addr, data)
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
                let success = target
                    .write_addrs(Self::to_target_usize(cmd.addr)?, cmd.val)
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

                use BreakOp::*;
                let supported = match cmd.type_ {
                    0 => target
                        .update_sw_breakpoint(addr, Add)
                        .map(|_| true)
                        .map_err(MaybeUnimpl::error),
                    1 => target.update_hw_breakpoint(addr, Add),
                    2 => target.update_hw_watchpoint(addr, Add, WatchKind::Write),
                    3 => target.update_hw_watchpoint(addr, Add, WatchKind::Read),
                    4 => target.update_hw_watchpoint(addr, Add, WatchKind::ReadWrite),
                    // only 5 documented types in the protocol
                    _ => Err(MaybeUnimpl::no_impl()),
                }
                .maybe_missing_impl()?;

                match supported {
                    None => {}
                    Some(true) => res.write_str("OK")?,
                    Some(false) => res.write_str("E22")?, // value of 22 grafted from QEMU
                }
            }
            Command::z(cmd) => {
                let addr = Self::to_target_usize(cmd.addr)?;

                use BreakOp::*;
                let supported = match cmd.type_ {
                    0 => target
                        .update_sw_breakpoint(addr, Remove)
                        .map(|_| true)
                        .map_err(MaybeUnimpl::error),
                    1 => target.update_hw_breakpoint(addr, Remove),
                    2 => target.update_hw_watchpoint(addr, Remove, WatchKind::Write),
                    3 => target.update_hw_watchpoint(addr, Remove, WatchKind::Read),
                    4 => target.update_hw_watchpoint(addr, Remove, WatchKind::ReadWrite),
                    // only 5 documented types in the protocol
                    _ => Err(MaybeUnimpl::no_impl()),
                }
                .maybe_missing_impl()?;

                match supported {
                    None => {}
                    Some(true) => res.write_str("OK")?,
                    Some(false) => res.write_str("E22")?, // value of 22 grafted from QEMU
                }
            }
            Command::vCont(cmd) => {
                use crate::protocol::_vCont::VContKind;

                // map raw vCont action iterator to a format the `Target` expects
                let mut err = Ok(());
                let mut actions = cmd.actions.into_iter().filter_map(|action| {
                    let action = match action {
                        Ok(action) => action,
                        Err(e) => {
                            err = Err(e);
                            return None;
                        }
                    };

                    let resume_action = match action.kind {
                        VContKind::Step => ResumeAction::Step,
                        VContKind::Continue => ResumeAction::Continue,
                        // NOTE: this `unimplemented!` should never be hit so long as `vCont?`
                        // returns only currently implemented vCont actions.
                        _ => unimplemented!("unimplemented vCont action {:?}", action.kind),
                    };

                    let tid = match action.tid {
                        Some(tid) => tid.tid,
                        // An action with no thread-id matches all threads
                        None => TidSelector::Any,
                    };

                    Some((tid, resume_action))
                });

                let ret = self.do_vcont(res, target, &mut actions);
                err.map_err(|_| Error::PacketParse)?;
                return ret;
            }
            // TODO?: support custom resume addr in 'c' and 's'
            Command::c(_) => {
                return self.do_vcont(
                    res,
                    target,
                    &mut core::iter::once((self.current_tid.tid, ResumeAction::Continue)),
                )
            }
            Command::s(_) => {
                return self.do_vcont(
                    res,
                    target,
                    &mut core::iter::once((self.current_tid.tid, ResumeAction::Step)),
                )
            }

            // ------------------- Multi-threading Support ------------------ //
            Command::H(cmd) => {
                self.current_tid = cmd.tid;
                match self.current_tid.tid {
                    TidSelector::WithID(id) => {
                        target.set_current_thread(id).maybe_missing_impl()?;
                    }
                    // FIXME: this seems kinda sketchy
                    TidSelector::Any => {}
                    TidSelector::All => {}
                }

                res.write_str("OK")?
            }
            Command::qfThreadInfo(_) => {
                res.write_str("m")?;

                let mut err: Result<_, Error<T::Error, C::Error>> = Ok(());
                let mut first = true;
                target
                    .list_active_threads(&mut |tid| {
                        // TODO: replace this with a try block (once stabilized)
                        let e = (|| {
                            if !first {
                                self.multithread = true;
                                res.write_str(",")?
                            }
                            first = false;
                            res.write_num(tid.get())?;
                            Ok(())
                        })();

                        if let Err(e) = e {
                            err = Err(e)
                        }
                    })
                    .map_err(Error::TargetError)?;
                err?;
            }
            Command::qsThreadInfo(_) => res.write_str("l")?,
            Command::qC(_) => {
                res.write_str("QC")?;
                res.write_tid(self.current_tid)?;
            }
            Command::T(cmd) => {
                let alive = match cmd.tid.tid {
                    TidSelector::WithID(tid) => target
                        .is_thread_alive(tid)
                        .maybe_missing_impl()?
                        .unwrap_or(true),
                    // FIXME: this is pretty sketch :/
                    _ => unimplemented!(),
                };
                if alive {
                    res.write_str("OK")?;
                } else {
                    res.write_str("E00")?; // TODO: is this an okay error code?
                }
            }

            // ------------------ "Extended" Functionality ------------------ //
            Command::qRcmd(cmd) => {
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

                let supported = target
                    .handle_monitor_cmd(cmd.hex_cmd, ConsoleOutput::new(&mut callback))
                    .maybe_missing_impl()?;
                err?;

                if supported.is_some() {
                    res.write_str("OK")?
                }
            }

            // -------------------------------------------------------------- //
            Command::Unknown(cmd) => info!("Unknown command: {}", cmd),
            #[allow(unreachable_patterns)]
            c => warn!("Unimplemented command: {:?}", c),
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
        let (tid, stop_reason) = target
            .resume(Actions::new(actions), &mut || match res.as_conn().peek() {
                Ok(Some(0x03)) => true, // 0x03 is the interrupt byte
                Ok(Some(_)) => false,   // it's nothing that can't wait...
                Ok(None) => false,
                Err(e) => {
                    err = Err(Error::ConnectionRead(e));
                    true // break ASAP if a connection error occurred
                }
            })
            .map_err(Error::TargetError)?;
        err?;

        self.current_tid.tid = TidSelector::WithID(tid);
        target.set_current_thread(tid).maybe_missing_impl()?;

        match stop_reason {
            StopReason::DoneStep | StopReason::GdbInterrupt => {
                res.write_str("S05")?;
                Ok(None)
            }
            StopReason::Signal(code) => {
                res.write_str("S")?;
                res.write_num(code)?;
                Ok(None)
            }
            StopReason::Halted => {
                res.write_str("W00")?;
                Ok(Some(DisconnectReason::TargetHalted))
            }
            stop_reason => {
                // otherwise, a breakpoint was hit

                res.write_str("T05")?;

                res.write_str("thread:")?;
                res.write_tid(self.current_tid)?;
                res.write_str(";")?;

                match stop_reason {
                    // don't include addr on sw/hw break
                    StopReason::SwBreak => res.write_str("swbreak:")?,
                    StopReason::HwBreak => res.write_str("hwbreak:")?,
                    StopReason::Watch { kind, addr } => {
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
        // TODO?: more granular error when GDB sends a number which is too big?
        let mut buf = [0; 16];
        let len = n.to_be_bytes(&mut buf).ok_or(Error::PacketParse)?;
        <T::Arch as Arch>::Usize::from_be_bytes(&buf[..len]).ok_or(Error::PacketParse)
    }
}
