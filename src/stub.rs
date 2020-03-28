use alloc::collections::BTreeSet;
use alloc::vec::Vec;
use core::marker::PhantomData;

use log::*;

use crate::{
    protocol::{Command, Packet, ResponseWriter},
    Access, AccessKind, Connection, Error, Target, TargetState,
};

enum ExecState {
    Paused,
    Running { single_step: bool },
    Exit,
}

/// Facilitates the remote debugging of a [`Target`](trait.Target.html) using
/// the GDB Remote Serial Protocol over a given
/// [`Connection`](trait.Connection.html).
pub struct GdbStub<T: Target, C: Connection> {
    conn: C,
    exec_state: ExecState,
    swbreak: BTreeSet<T::Usize>,
    hwbreak: BTreeSet<T::Usize>,
    wwatch: BTreeSet<T::Usize>,
    rwatch: BTreeSet<T::Usize>,
    awatch: BTreeSet<T::Usize>,
    _target: PhantomData<T>,
}

impl<T: Target, C: Connection> GdbStub<T, C> {
    /// Create a new `GdbStub` using the provided connection.
    pub fn new(conn: C) -> GdbStub<T, C> {
        GdbStub {
            conn,
            swbreak: BTreeSet::new(),
            hwbreak: BTreeSet::new(),
            wwatch: BTreeSet::new(),
            rwatch: BTreeSet::new(),
            awatch: BTreeSet::new(),
            exec_state: ExecState::Paused,
            _target: PhantomData,
        }
    }

    fn handle_command(&mut self, target: &mut T, command: Command) -> Result<(), Error<T, C>> {
        // Acknowledge the command
        self.conn.write(b'+').map_err(Error::ConnectionRead)?;

        let mut res = ResponseWriter::new(&mut self.conn);

        match command {
            // ------------------ Handshaking and Queries ------------------- //
            Command::qSupported(_features) => {
                // TODO: enumerate qSupported features better
                res.write_str("swbreak+;")?;
                res.write_str("hwbreak+;")?;
                res.write_str("vContSupported+;")?;

                if T::target_description_xml().is_some() {
                    res.write_str("qXfer:features:read+;")?;
                }
            }
            Command::vContQuestionMark(_) => res.write_str("vCont;c;C;s;S;t")?,
            Command::qXferFeaturesRead(cmd) => {
                let _annex = cmd.annex; // This _should_ always be target.xml...
                match T::target_description_xml() {
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
                let mut err = Ok(());
                target.read_registers(|reg| {
                    if let Err(e) = res.write_hex_buf(reg) {
                        err = Err(e)
                    }
                });
                err?;
            }
            Command::G(cmd) => {
                // TODO: use the length of the slice returned by `target.read_registers` to
                // validate that the server sent the correct amount of data
                target.write_registers(cmd.vals.as_slice());
                res.write_str("OK")?;
            }
            Command::m(cmd) => {
                let mut err = Ok(());
                // XXX: get rid of these unwraps ahhh
                let start = num_traits::NumCast::from(cmd.addr).unwrap();
                let end = num_traits::NumCast::from(cmd.addr + cmd.len as u64).unwrap();

                target.read_addrs(start..end, |val| {
                    // TODO: assert the length is correct
                    if let Err(e) = res.write_hex(val) {
                        err = Err(e)
                    }
                });
                err?;
            }
            Command::M(cmd) => {
                let addr = cmd.addr;
                let mut val = cmd
                    .val
                    .into_iter()
                    .enumerate()
                    .map(|(i, v)| (addr + i as u64, v))
                    // XXX: get rid of this unwrap ahhh
                    .map(|(i, v)| (num_traits::NumCast::from(i).unwrap(), v));

                target.write_addrs(|| val.next());
            }
            Command::D(_) => {
                res.write_str("OK")?;
                self.exec_state = ExecState::Exit
            }
            Command::Z(cmd) => {
                // XXX: get rid of this unwrap ahhh
                let addr = num_traits::NumCast::from(cmd.addr).unwrap();
                let supported = match cmd.type_ {
                    // TODO: defer implementation of hardware and software breakpoints to target
                    0 => Some(self.swbreak.insert(addr)),
                    1 => Some(self.hwbreak.insert(addr)),
                    2 => Some(self.wwatch.insert(addr)),
                    3 => Some(self.rwatch.insert(addr)),
                    4 => Some(self.awatch.insert(addr)),
                    _ => None,
                };

                if supported.is_some() {
                    res.write_str("OK")?;
                }
            }
            Command::z(cmd) => {
                // XXX: get rid of this unwrap ahhh
                let addr = num_traits::NumCast::from(cmd.addr).unwrap();
                let supported = match cmd.type_ {
                    // TODO: defer implementation of hardware and software breakpoints to target
                    0 => Some(self.swbreak.remove(&addr)),
                    1 => Some(self.hwbreak.remove(&addr)),
                    2 => Some(self.wwatch.remove(&addr)),
                    3 => Some(self.rwatch.remove(&addr)),
                    4 => Some(self.awatch.remove(&addr)),
                    _ => None,
                };

                if supported.is_some() {
                    res.write_str("OK")?;
                }
            }
            Command::vCont(cmd) => {
                use crate::protocol::_vCont::VContKind;
                let action = &cmd.actions[0];
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

    fn recv_packet<'a, 'b>(
        &'a mut self,
        packet_buffer: &'b mut Vec<u8>,
    ) -> Result<Option<Packet<'b>>, Error<T, C>> {
        let header_byte = match self.exec_state {
            // block waiting for a gdb command
            ExecState::Paused => self.conn.read().map(Some),
            ExecState::Running { .. } => self.conn.read_nonblocking(),
            ExecState::Exit => unreachable!(),
        };

        match header_byte {
            Ok(None) => Ok(None), // no incoming message
            Ok(Some(header_byte)) => {
                packet_buffer.clear();
                packet_buffer.push(header_byte);
                if header_byte == b'$' {
                    // read the packet body
                    loop {
                        match self.conn.read().map_err(Error::ConnectionRead)? {
                            b'#' => break,
                            x => packet_buffer.push(x),
                        }
                    }
                    // append the # char
                    packet_buffer.push(b'#');
                    // and finally, read the checksum as well
                    packet_buffer.push(self.conn.read().map_err(Error::ConnectionRead)?);
                    packet_buffer.push(self.conn.read().map_err(Error::ConnectionRead)?);
                }

                match Packet::from_buf(packet_buffer) {
                    Ok(packet) => Ok(Some(packet)),
                    Err(e) => {
                        error!("Could not parse packet: {:?}", e);
                        Err(Error::PacketParse)
                    }
                }
            }
            Err(e) => Err(Error::ConnectionRead(e)),
        }
    }

    fn send_breakpoint_stop_response(
        &mut self,
        stop_reason: StopReason<T::Usize>,
    ) -> Result<(), Error<T, C>> {
        let mut res = ResponseWriter::new(&mut self.conn);

        res.write_str("T")?;
        res.write_hex(5)?;

        macro_rules! write_addr {
            ($addr:expr) => {
                // XXX: get rid of this unwrap ahhh
                let addr: u64 = num_traits::NumCast::from($addr).unwrap();
                res.write_hex_buf(&addr.to_ne_bytes())?;
            };
        }

        match stop_reason {
            StopReason::WWatch(addr) => {
                res.write_str("watch:")?;
                write_addr!(addr);
            }
            StopReason::RWatch(addr) => {
                res.write_str("rwatch:")?;
                write_addr!(addr);
            }
            StopReason::AWatch(addr) => {
                res.write_str("awatch:")?;
                write_addr!(addr);
            }
            StopReason::SwBreak => res.write_str("swbreak:")?,
            StopReason::HwBreak => res.write_str("hwbreak:")?,
        };

        res.write_str(";")?;

        Ok(res.flush()?)
    }

    /// Starts a GDB remote debugging session.
    ///
    /// Returns once the GDB client cleanly closes the debugging session (via
    /// the GDB `quit` command).
    pub fn run(&mut self, target: &mut T) -> Result<TargetState, Error<T, C>> {
        let mut packet_buffer = Vec::new();

        loop {
            // Handle any incoming GDB packets
            match self.recv_packet(&mut packet_buffer)? {
                None => {}
                Some(packet) => match packet {
                    Packet::Ack => {}
                    Packet::Nack => unimplemented!(),
                    Packet::Interrupt => {
                        self.exec_state = ExecState::Paused;
                        let mut res = ResponseWriter::new(&mut self.conn);
                        res.write_str("S05")?;
                        res.flush()?;
                    }
                    Packet::Command(command) => {
                        self.handle_command(target, command)?;
                    }
                },
            };

            match self.exec_state {
                ExecState::Paused => {}
                ExecState::Exit => {
                    return Ok(TargetState::Running);
                }
                ExecState::Running { single_step } => {
                    let mut stop_reason = None;

                    // check for memory breakpoints on each access
                    let on_access = |access: Access<T::Usize>| {
                        if self.awatch.contains(&access.addr) {
                            stop_reason = Some(StopReason::AWatch(access.addr));
                            return;
                        }

                        match access.kind {
                            AccessKind::Read => {
                                if self.rwatch.contains(&access.addr) {
                                    stop_reason = Some(StopReason::RWatch(access.addr))
                                }
                            }
                            AccessKind::Write => {
                                if self.wwatch.contains(&access.addr) {
                                    stop_reason = Some(StopReason::WWatch(access.addr))
                                }
                            }
                        }
                    };

                    let target_state = target.step(on_access).map_err(Error::TargetError)?;
                    if target_state == TargetState::Halted {
                        // "The process exited with status code 00"
                        let mut res = ResponseWriter::new(&mut self.conn);
                        res.write_str("W00")?;
                        res.flush()?;
                        return Ok(TargetState::Halted);
                    };

                    // TODO: defer implementation of hardware and software breakpoints to target
                    let target_pc = target.read_pc();
                    if target_state == TargetState::Breakpoint || self.swbreak.contains(&target_pc)
                    {
                        stop_reason = Some(StopReason::SwBreak)
                    } else if self.hwbreak.contains(&target_pc) {
                        stop_reason = Some(StopReason::HwBreak)
                    }

                    // if something interesting happened, send the stop response
                    if let Some(stop_reason) = stop_reason {
                        warn!("{:x?}", stop_reason);
                        self.send_breakpoint_stop_response(stop_reason)?;
                        self.exec_state = ExecState::Paused;
                    } else if single_step {
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

#[derive(Debug)]
enum StopReason<U> {
    WWatch(U),
    RWatch(U),
    AWatch(U),
    SwBreak,
    HwBreak,
    // TODO: add more stop reasons
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
