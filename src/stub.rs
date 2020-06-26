use num_traits::ops::saturating::Saturating;

use crate::{
    arch_traits::{Arch, Registers},
    connection::Connection,
    error::Error,
    protocol::{Command, Packet, ResponseWriter, Tid, TidKind},
    target::{BreakOp, ResumeAction, StopReason, Target, WatchKind},
    util::slicevec::SliceVec,
};

/// Reason the GDB session has ended
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DisconnectReason {
    /// Target Halted
    TargetHalted,
    /// GDB issued a disconnect command
    Disconnect,
    /// GDB issued a kill command
    Kill,
}

/// Facilitates the remote debugging of a [`Target`](trait.Target.html) using
/// the GDB Remote Serial Protocol over a given
/// [`Connection`](trait.Connection.html).
pub struct GdbStub<'a, T: Target, C: Connection> {
    conn: Option<C>,
    packet_buffer: Option<&'a mut [u8]>,

    current_tid: Tid,
    _target: core::marker::PhantomData<T>,
}

impl<'a, T: Target, C: Connection> GdbStub<'a, T, C> {
    /// Create a new `GdbStub` using the provided connection + packet buffer.
    pub fn new(conn: C, packet_buffer: &'a mut [u8]) -> GdbStub<T, C> {
        GdbStub {
            conn: Some(conn),
            packet_buffer: Some(packet_buffer),

            current_tid: Tid {
                pid: None,
                tid: TidKind::Any,
            },
            _target: core::marker::PhantomData,
        }
    }

    fn do_vcont(
        &mut self,
        res: &mut ResponseWriter<C>,
        target: &mut T,
        actions: impl Iterator<Item = (TidKind, ResumeAction)>,
    ) -> Result<Option<DisconnectReason>, Error<T, C>> {
        let stop_reason = target.resume(actions).map_err(Error::TargetError)?;

        // if the target isn't running, send a stop-response packet
        if stop_reason != StopReason::Running {
            // if the target Halted, send a "process exited with status code 0" packet,
            // and break the loop.
            if stop_reason == StopReason::Halted {
                res.write_str("W00")?;
                return Ok(Some(DisconnectReason::TargetHalted));
            }

            // if the target's still running, just say it was a signal 05
            if stop_reason == StopReason::Running {
                res.write_str("S05")?;
                return Ok(None);
            }

            // otherwise, a breakpoint was hit
            res.write_str("T")?;
            res.write_hex(5)?;

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
                    // XXX: get rid of this unwrap ahhh
                    let addr: u64 = num_traits::NumCast::from(addr).unwrap();
                    res.write_hex_buf(&addr.to_be_bytes())?;
                }
                _ => unreachable!(),
            };

            res.write_str(";")?;
        }

        Ok(None)
    }

    fn handle_command(
        &mut self,
        res: &mut ResponseWriter<C>,
        target: &mut T,
        command: Command<'_>,
    ) -> Result<Option<DisconnectReason>, Error<T, C>> {
        match command {
            // ------------------ Handshaking and Queries ------------------- //
            Command::qSupported(cmd) => {
                let _features = cmd.features.into_iter();

                res.write_str("vContSupported+;")?;
                res.write_str("multiprocess+;")?;
                res.write_str("swbreak+;")?;

                // probe support for various watchpoints/breakpoints
                let mut supports_hwbreak = false;

                let test_addr = num_traits::NumCast::from(0).unwrap();
                if (target.update_hw_breakpoint(test_addr, BreakOp::Add)).is_some() {
                    target.update_hw_breakpoint(test_addr, BreakOp::Remove);
                    supports_hwbreak = true;
                }
                if (target.update_hw_watchpoint(test_addr, BreakOp::Add, WatchKind::Write))
                    .is_some()
                {
                    target.update_hw_watchpoint(test_addr, BreakOp::Remove, WatchKind::Write);
                    supports_hwbreak = true;
                }

                if supports_hwbreak {
                    res.write_str("hwbreak+;")?;
                }

                // TODO: implement conditional breakpoint support (since that's kool).
                // res.write_str("ConditionalBreakpoints+;")?;

                // probe support for target description xml
                if T::Arch::target_description_xml().is_some() {
                    res.write_str("qXfer:features:read+;")?;
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
            // TODO: Improve the '?' response...
            Command::QuestionMark(_) => res.write_str("S05")?,
            Command::qAttached(_) => res.write_str("1")?, // attached to existing process
            Command::g(_) => {
                let mut regs: <T::Arch as Arch>::Registers = Default::default();
                target
                    .read_registers(&mut regs)
                    .map_err(Error::TargetError)?;
                let mut err = Ok(());
                regs.gdb_serialize(|val| {
                    let res = match val {
                        Some(b) => res.write_hex(b),
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
                    .map_err(|_| Error::PacketParse)?; // FIXME: more granular error?
                target.write_registers(&regs).map_err(Error::TargetError)?;
                res.write_str("OK")?;
            }
            Command::m(cmd) => {
                let mut err = Ok(());
                // XXX: get rid of these unwraps ahhh
                let start: <T::Arch as Arch>::Usize = num_traits::NumCast::from(cmd.addr).unwrap();
                // XXX: on overflow, this _should_ wrap around to low addresses (maybe?)
                let end = start.saturating_add(num_traits::NumCast::from(cmd.len).unwrap());

                target
                    .read_addrs(start..end, |val| {
                        // TODO: assert the length is correct
                        if let Err(e) = res.write_hex(val) {
                            err = Err(e)
                        }
                    })
                    .map_err(Error::TargetError)?;
                err?;
            }
            Command::M(cmd) => {
                let addr = cmd.addr;
                let mut val = cmd
                    .val
                    .enumerate()
                    .map(|(i, v)| (addr + i as u64, v))
                    // XXX: get rid of this unwrap ahhh
                    .map(|(i, v)| (num_traits::NumCast::from(i).unwrap(), v));

                target
                    .write_addrs(|| val.next())
                    .map_err(Error::TargetError)?;
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
                // XXX: get rid of this unwrap ahhh
                let addr: <T::Arch as Arch>::Usize = num_traits::NumCast::from(cmd.addr).unwrap();

                use BreakOp::*;
                let supported = match cmd.type_ {
                    0 => Some(target.update_sw_breakpoint(addr, Add).map(|_| true)),
                    1 => target.update_hw_breakpoint(addr, Add),
                    2 => target.update_hw_watchpoint(addr, Add, WatchKind::Write),
                    3 => target.update_hw_watchpoint(addr, Add, WatchKind::Read),
                    4 => target.update_hw_watchpoint(addr, Add, WatchKind::ReadWrite),
                    // only 5 documented types in the protocol
                    _ => None,
                };

                match supported {
                    None => {}
                    Some(Ok(true)) => res.write_str("OK")?,
                    Some(Ok(false)) => res.write_str("E22")?, // value of 22 grafted from QEMU
                    Some(Err(e)) => return Err(Error::TargetError(e)),
                }
            }
            Command::z(cmd) => {
                // XXX: get rid of this unwrap ahhh
                let addr: <T::Arch as Arch>::Usize = num_traits::NumCast::from(cmd.addr).unwrap();

                use BreakOp::*;
                let supported = match cmd.type_ {
                    0 => Some(target.update_sw_breakpoint(addr, Remove).map(|_| true)),
                    1 => target.update_hw_breakpoint(addr, Remove),
                    2 => target.update_hw_watchpoint(addr, Remove, WatchKind::Write),
                    3 => target.update_hw_watchpoint(addr, Remove, WatchKind::Read),
                    4 => target.update_hw_watchpoint(addr, Remove, WatchKind::ReadWrite),
                    // only 5 documented types in the protocol
                    _ => None,
                };

                match supported {
                    None => {}
                    Some(Ok(true)) => res.write_str("OK")?,
                    Some(Ok(false)) => res.write_str("E22")?, // value of 22 grafted from QEMU
                    Some(Err(e)) => return Err(Error::TargetError(e)),
                }
            }
            Command::vCont(cmd) => {
                use crate::protocol::_vCont::VContKind;

                // map raw vCont action iterator to a format the `Target` expects
                let mut err = Ok(());
                let actions = cmd.actions.into_iter().filter_map(|action| {
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
                        _ => unimplemented!("unimplemented vCont action {:?}", action.kind),
                    };

                    let tid = match action.tid {
                        Some(tid) => tid.tid,
                        // An action with no thread-id matches all threads
                        None => TidKind::Any,
                    };

                    Some((tid, resume_action))
                });

                let ret = self.do_vcont(res, target, actions);
                err.map_err(|_| Error::PacketParse)?;
                return ret;
            }
            // TODO?: support custom resume addr in 'c' and 's'
            Command::c(_) => {
                self.do_vcont(
                    res,
                    target,
                    core::iter::once((self.current_tid.tid, ResumeAction::Continue)),
                )?;
            }
            Command::s(_) => {
                self.do_vcont(
                    res,
                    target,
                    core::iter::once((self.current_tid.tid, ResumeAction::Step)),
                )?;
            }

            // ------------------- Stubbed Functionality -------------------- //
            // TODO: add proper support for >1 "thread"
            // for now, just hard-code a single thread with id 1
            Command::H(cmd) => {
                self.current_tid = cmd.tid;
                res.write_str("OK")?
            }
            Command::qfThreadInfo(_) => res.write_str("m1")?,
            Command::qsThreadInfo(_) => res.write_str("l")?,
            Command::qC(_) => res.write_str("QC1")?,

            // -------------------------------------------------------------- //
            Command::Unknown(cmd) => warn!("Unknown command: {}", cmd),
            #[allow(unreachable_patterns)]
            c => warn!("Unimplemented command: {:?}", c),
        }

        Ok(None)
    }

    fn recv_packet<'b>(
        &mut self,
        conn: &mut C,
        buf: &'b mut [u8],
        blocking: bool,
    ) -> Result<Option<Packet<'b>>, Error<T, C>> {
        let header_byte = if blocking {
            conn.read().map(Some)
        } else {
            conn.read_nonblocking()
        };

        match header_byte {
            Ok(None) => Ok(None), // no incoming message
            Ok(Some(header_byte)) => {
                // use SliceVec as a convenient view into the packet buffer
                let mut packet_buffer = SliceVec::new(buf);

                packet_buffer.clear();
                packet_buffer.push(header_byte)?;
                if header_byte == b'$' {
                    // read the packet body
                    loop {
                        let c = conn.read().map_err(Error::ConnectionRead)?;
                        packet_buffer.push(c)?;
                        if c == b'#' {
                            break;
                        }
                    }
                    // read the checksum as well
                    packet_buffer.push(conn.read().map_err(Error::ConnectionRead)?)?;
                    packet_buffer.push(conn.read().map_err(Error::ConnectionRead)?)?;
                }

                let len = packet_buffer.len();
                drop(packet_buffer);

                match Packet::from_buf(&buf[..len]) {
                    Ok(packet) => Ok(Some(packet)),
                    Err(e) => {
                        // TODO: preserve this context within Error::PacketParse
                        error!("Could not parse packet: {:?}", e);
                        Err(Error::PacketParse)
                    }
                }
            }
            Err(e) => Err(Error::ConnectionRead(e)),
        }
    }

    /// Starts a GDB remote debugging session.
    ///
    /// Returns once the GDB client closes the debugging session, or if the
    /// target halts.
    pub fn run(&mut self, target: &mut T) -> Result<DisconnectReason, Error<T, C>> {
        let packet_buffer = self.packet_buffer.take().unwrap();
        let mut conn = self.conn.take().unwrap();

        loop {
            // Handle any incoming GDB packets
            match self.recv_packet(&mut conn, packet_buffer, true)? {
                None => {}
                Some(packet) => match packet {
                    Packet::Ack => {}
                    Packet::Nack => {
                        unimplemented!("GDB nack'd the packet, but retry isn't implemented yet")
                    }
                    Packet::Interrupt => unimplemented!(),
                    Packet::Command(command) => {
                        // Acknowledge the command
                        conn.write(b'+').map_err(Error::ConnectionRead)?;

                        let mut res = ResponseWriter::new(&mut conn);
                        let status = self.handle_command(&mut res, target, command)?;

                        // HACK: this could be more elegant...
                        if status != Some(DisconnectReason::Kill) {
                            res.flush()?;
                        }

                        if let Some(disconnect_reason) = status {
                            return Ok(disconnect_reason);
                        }
                    }
                },
            };
        }
    }
}

// enum SignalMetadata {
//     Register(u8, Vec<u8>),
//     Thread { tid: isize },
//     Core(usize),
//     StopReason(StopReason),
// }

// enum StopReply<'a> {
//     Signal(u8),                              // S
//     SignalWithMeta(u8, Vec<SignalMetadata>), // T
//     Exited {
//         status: u8,
//         pid: Option<isize>,
//     }, // W
//     Terminated {
//         status: u8,
//         pid: Option<isize>,
//     }, // X
//     ThreadExit {
//         status: u8,
//         tid: isize,
//     }, // w
//     NoResumedThreads,                        // N
//     ConsoleOutput(&'a [u8]),                 // O
//     FileIOSyscall {
//         call_id: &'a str,
//         params: Vec<&'a str>,
//     },
// }
