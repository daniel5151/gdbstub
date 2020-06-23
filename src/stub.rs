use arrayvec::ArrayVec;
use log::*;
use num_traits::ops::saturating::Saturating;

use crate::util::slicevec::SliceVec;
use crate::{
    protocol::{Command, Packet, ResponseWriter},
    Arch, Connection, Error, HwBreakOp, Registers, Target, TargetState, WatchKind,
};

enum ExecState {
    Paused,
    Running { single_step: bool },
    Exit,
}

// TODO?: make number of swbreak configurable
const MAX_SWBREAK: usize = 32;

/// Facilitates the remote debugging of a [`Target`](trait.Target.html) using
/// the GDB Remote Serial Protocol over a given
/// [`Connection`](trait.Connection.html).
pub struct GdbStub<'a, T: Target, C: Connection> {
    conn: C,
    exec_state: ExecState,
    // TODO?: use BTreeSet if std is enabled for infinite swbreaks?
    swbreak: ArrayVec<[<T::Arch as Arch>::Usize; MAX_SWBREAK]>,
    packet_buffer: Option<&'a mut [u8]>,
}

impl<'a, T: Target, C: Connection> GdbStub<'a, T, C> {
    /// Create a new `GdbStub` using the provided connection + packet buffer.
    pub fn new(conn: C, packet_buffer: &'a mut [u8]) -> GdbStub<T, C> {
        GdbStub {
            conn,
            swbreak: ArrayVec::new(),
            exec_state: ExecState::Paused,
            packet_buffer: Some(packet_buffer),
        }
    }

    fn handle_command(&mut self, target: &mut T, command: Command<'_>) -> Result<(), Error<T, C>> {
        // Acknowledge the command
        self.conn.write(b'+').map_err(Error::ConnectionRead)?;

        let mut res = ResponseWriter::new(&mut self.conn);

        match command {
            // ------------------ Handshaking and Queries ------------------- //
            Command::qSupported(cmd) => {
                let _features = cmd.features.into_iter();

                res.write_str("swbreak+;")?;
                res.write_str("vContSupported+;")?;

                // probe support for hw breakpoints
                let test_addr = num_traits::NumCast::from(0).unwrap();
                let can_set_hw_break = target.update_hw_breakpoint(test_addr, HwBreakOp::AddBreak);
                if can_set_hw_break.is_some() {
                    target.update_hw_breakpoint(test_addr, HwBreakOp::RemoveBreak);

                    res.write_str("hwbreak+;")?;
                    res.write_str("BreakpointCommands+;")?;
                }

                // TODO: implement conditional breakpoint support (since that's kool).
                // res.write_str("ConditionalBreakpoints+;")?;

                // probe support for target description xml
                if T::Arch::target_description_xml().is_some() {
                    res.write_str("qXfer:features:read+;")?;
                }
            }
            Command::vContQuestionMark(_) => res.write_str("vCont;c;s;t")?,
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
                    None => unreachable!(),
                }
            }

            // -------------------- "Core" Functionality -------------------- //
            // TODO: Improve the '?' response...
            Command::QuestionMark(_) => res.write_str("S05")?,
            Command::qAttached(_) => res.write_str("1")?,
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
            Command::D(_) => {
                res.write_str("OK")?;
                self.exec_state = ExecState::Exit
            }
            Command::Z(cmd) => {
                // XXX: get rid of this unwrap ahhh
                let addr: <T::Arch as Arch>::Usize = num_traits::NumCast::from(cmd.addr).unwrap();

                use HwBreakOp::*;
                let supported = match cmd.type_ {
                    0 => Some(Ok(self.swbreak.try_push(addr).is_ok())),
                    1 => target.update_hw_breakpoint(addr, AddBreak),
                    2 => target.update_hw_breakpoint(addr, AddWatch(WatchKind::Write)),
                    3 => target.update_hw_breakpoint(addr, AddWatch(WatchKind::Read)),
                    4 => target.update_hw_breakpoint(addr, AddWatch(WatchKind::ReadWrite)),
                    _ => None, // only 5 documented types in the protocol
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

                use HwBreakOp::*;
                let supported = match cmd.type_ {
                    0 => match self.swbreak.iter().position(|x| x == &addr) {
                        Some(i) => {
                            self.swbreak.swap_remove(i);
                            Some(Ok(true))
                        }
                        None => Some(Ok(false)),
                    },
                    1 => target.update_hw_breakpoint(addr, RemoveBreak),
                    2 => target.update_hw_breakpoint(addr, RemoveWatch(WatchKind::Write)),
                    3 => target.update_hw_breakpoint(addr, RemoveWatch(WatchKind::Read)),
                    4 => target.update_hw_breakpoint(addr, RemoveWatch(WatchKind::ReadWrite)),
                    _ => None, // only 5 documented types in the protocol
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
                // XXX: handle multiple actions properly
                let action = cmd.actions.into_iter().next().unwrap().unwrap();
                self.exec_state = match action.kind {
                    VContKind::Step => ExecState::Running { single_step: true },
                    VContKind::Continue => ExecState::Running { single_step: false },
                    _ => unimplemented!("unsupported vCont action"),
                };
                // no immediate response
                return Ok(());
            }
            // TODO?: support custom resume addr in 'c' and 's'
            Command::c(_) => {
                self.exec_state = ExecState::Running { single_step: false };
                // no immediate response
                return Ok(());
            }
            Command::s(_) => {
                self.exec_state = ExecState::Running { single_step: true };
                // no immediate response
                return Ok(());
            }

            // ------------------- Stubbed Functionality -------------------- //
            // TODO: add proper support for >1 "thread"
            // for now, just hard-code a single thread with id 1
            Command::H(_) => res.write_str("OK")?,
            Command::qfThreadInfo(_) => res.write_str("m1")?,
            Command::qsThreadInfo(_) => res.write_str("l")?,
            Command::qC(_) => res.write_str("QC1")?,

            // -------------------------------------------------------------- //
            Command::Unknown(cmd) => warn!("Unknown command: {}", cmd),
            #[allow(unreachable_patterns)]
            c => warn!("Unimplemented command: {:?}", c),
        }

        res.flush().map_err(Error::ConnectionWrite)
    }

    fn recv_packet<'b>(&mut self, buf: &'b mut [u8]) -> Result<Option<Packet<'b>>, Error<T, C>> {
        let header_byte = match self.exec_state {
            // block waiting for a gdb command
            ExecState::Paused => self.conn.read().map(Some),
            ExecState::Running { .. } => self.conn.read_nonblocking(),
            ExecState::Exit => unreachable!(),
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
                        let c = self.conn.read().map_err(Error::ConnectionRead)?;
                        packet_buffer.push(c)?;
                        if c == b'#' {
                            break;
                        }
                    }
                    // read the checksum as well
                    packet_buffer.push(self.conn.read().map_err(Error::ConnectionRead)?)?;
                    packet_buffer.push(self.conn.read().map_err(Error::ConnectionRead)?)?;
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
    /// Returns once the GDB client cleanly closes the debugging session (via
    /// the GDB `quit` command).
    pub fn run(&mut self, target: &mut T) -> Result<TargetState<T::Arch>, Error<T, C>> {
        let packet_buffer = self.packet_buffer.take().unwrap();

        loop {
            // Handle any incoming GDB packets
            match self.recv_packet(packet_buffer)? {
                None => {}
                Some(packet) => match packet {
                    Packet::Ack => {}
                    Packet::Nack => {
                        unimplemented!("GDB nack'd the packet, but retry isn't implemented yet")
                    }
                    Packet::Interrupt => {
                        self.exec_state = ExecState::Paused;
                        let mut res = ResponseWriter::new(&mut self.conn);
                        res.write_str("S05")?;
                        res.flush()?;
                    }
                    Packet::Command(command) => self.handle_command(target, command)?,
                },
            };

            match self.exec_state {
                ExecState::Paused => {}
                ExecState::Exit => return Ok(TargetState::Running),
                ExecState::Running { single_step } => {
                    let mut target_state = target.step().map_err(Error::TargetError)?;

                    // check if a software breakpoint was hit
                    let target_pc = target.read_pc().map_err(Error::TargetError)?;
                    if self.swbreak.contains(&target_pc) {
                        target_state = TargetState::SwBreak;
                    }

                    // if the target isn't running, send a stop-response packet
                    if target_state != TargetState::Running {
                        debug!("[0x{:x}] {:x?}", target_pc, target_state);

                        let mut res = ResponseWriter::new(&mut self.conn);

                        // if the target Halted, send a "process exited with status code 0" packet,
                        // and break the loop.
                        if target_state == TargetState::Halted {
                            res.write_str("W00")?;
                            res.flush()?;
                            return Ok(TargetState::Halted);
                        }

                        // otherwise, a breakpoint was hit
                        res.write_str("T")?;
                        res.write_hex(5)?;

                        match target_state {
                            // don't include addr on sw/hw break
                            TargetState::SwBreak => res.write_str("swbreak:")?,
                            TargetState::HwBreak => res.write_str("hwbreak:")?,
                            TargetState::Watch { kind, addr } => {
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
                        res.flush()?;

                        self.exec_state = ExecState::Paused;
                        continue;
                    }

                    if single_step {
                        self.exec_state = ExecState::Paused;
                        let mut res = ResponseWriter::new(&mut self.conn);
                        res.write_str("S05")?;
                        res.flush()?;
                    }
                }
            }
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
