use alloc::format;
use alloc::vec::Vec;

use log::*;

use crate::{
    protocol::{Command, Packet, ResponseWriter},
    Connection, Error, FromLEBytes, Target, TargetState,
};

enum ExecState {
    Paused,
    Running,
    Exit,
}

/// [`GdbStub`] maintains the state of a GDB remote debugging session, including
/// the underlying transport.
pub struct GdbStub<T: Target, C: Connection> {
    conn: C,
    exec_state: ExecState,
    _target: core::marker::PhantomData<T>,
}

impl<T: Target, C: Connection> GdbStub<T, C> {
    pub fn new(conn: C) -> GdbStub<T, C> {
        GdbStub {
            conn,
            exec_state: ExecState::Paused,
            _target: core::marker::PhantomData,
        }
    }

    fn handle_command(
        &mut self,
        target: &mut T,
        command: Command,
    ) -> Result<(), Error<T::Error, C::Error>> {
        // Acknowledge the command
        self.conn.write(b'+').map_err(Error::Connection)?;

        let mut res = ResponseWriter::new(&mut self.conn);

        match command {
            // ------------------ Handshaking and Queries ------------------- //
            Command::qSupported(_features) => {
                // TODO: properly enumerate own feature set
                res.write_str("BreakpointCommands+;swbreak+;vContSupported+;")?;

                if T::target_description_xml().is_some() {
                    res.write_str("qXfer:features:read+")?;
                }
            }
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
            // TODO: Improve the '?' response
            Command::QuestionMark(_) => res.write_str("S00")?,
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
            Command::m(m) => {
                let mut err = Ok(());
                // XXX: quick and dirty error handling, _not good_
                let start = T::Usize::from_le_bytes(&m.addr.to_le_bytes()).unwrap();
                let end = T::Usize::from_le_bytes(&(m.addr + m.len as u64).to_le_bytes()).unwrap();

                target.read_addrs(start..end, |val| {
                    // TODO: assert the length is correct
                    if let Err(e) = res.write_hex(val) {
                        err = Err(e)
                    }
                });
                err?;
            }
            Command::D(_) => {
                res.write_str("OK")?;
                self.exec_state = ExecState::Exit
            }

            // ------------------- Stubbed Functionality -------------------- //
            // TODO: add proper support for >1 "thread"
            // hard-code to return a single thread with id 1
            Command::H(_) => res.write_str("OK")?,
            Command::qfThreadInfo(_) => res.write_str("m1")?,
            Command::qsThreadInfo(_) => res.write_str("l")?,
            Command::qC(_) => res.write_str("QC1")?,

            // -------------------------------------------------------------- //
            Command::Unknown => trace!("Unknown command"),
            #[allow(unreachable_patterns)]
            c => trace!("Unimplemented command: {:?}", c),
        }

        res.flush().map_err(Error::ResponseConnection)
    }

    fn recv_packet<'a, 'b>(
        &'a mut self,
        packet_buffer: &'b mut Vec<u8>,
    ) -> Result<Option<Packet<'b>>, Error<T::Error, C::Error>> {
        let header_byte = match self.exec_state {
            // block waiting for a gdb command
            ExecState::Paused => self.conn.read().map(Some),
            ExecState::Running => self.conn.read_nonblocking(),
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
                        match self.conn.read().map_err(Error::Connection)? {
                            b'#' => break,
                            x => packet_buffer.push(x),
                        }
                    }
                    // append the # char
                    packet_buffer.push(b'#');
                    // and finally, read the checksum as well
                    packet_buffer.push(self.conn.read().map_err(Error::Connection)?);
                    packet_buffer.push(self.conn.read().map_err(Error::Connection)?);
                }

                Some(Packet::from_buf(packet_buffer))
                    .transpose()
                    .map_err(|e| Error::PacketParse(format!("{:?}", e)))
            }
            Err(e) => Err(Error::Connection(e)),
        }
    }

    /// Runs the target in a loop, with debug checks between each call to
    /// `target.step()`
    pub fn run(&mut self, target: &mut T) -> Result<TargetState, Error<T::Error, C::Error>> {
        let mut packet_buffer = Vec::new();
        let mut mem_accesses = Vec::new();

        loop {
            // Handle any incoming GDB packets
            match self.recv_packet(&mut packet_buffer)? {
                None => {}
                Some(packet) => match packet {
                    Packet::Ack => {}
                    Packet::Nack => unimplemented!(),
                    Packet::Command(command) => {
                        self.handle_command(target, command)?;
                    }
                },
            };

            match self.exec_state {
                ExecState::Paused => {}
                ExecState::Running => {
                    let target_state = target
                        .step(|access| mem_accesses.push(access))
                        .map_err(Error::TargetError)?;

                    if target_state == TargetState::Halted {
                        return Ok(TargetState::Halted);
                    };
                }
                ExecState::Exit => {
                    return Ok(TargetState::Running);
                }
            }
        }
    }
}
