use log::*;

use super::{Command, Connection, Error, Target, TargetState};

/// [`GdbStub`] maintains the state of a GDB remote debugging session, including
/// the underlying transport.
pub struct GdbStub<T: Target, C: Connection> {
    conn: C,
    paused: bool,
    _target: core::marker::PhantomData<T>,
}

impl<T: Target, C: Connection> GdbStub<T, C> {
    pub fn new(conn: C) -> GdbStub<T, C> {
        GdbStub {
            conn,
            paused: true,
            _target: core::marker::PhantomData,
        }
    }

    fn handle_command(
        &mut self,
        target: &mut T,
        command: Command,
    ) -> Result<(), Error<T::Error, C::Error>> {
        use Command::*;

        trace!("Handling {:?}", command);

        if command == Ack {
            // acknowledge command
            self.conn.write(b'+').map_err(Error::Connection)?;
            return Ok(());
        }

        let response = ResponseWriter::begin(&mut self.conn).map_err(Error::Connection)?;

        match command {
            Retransmit => unimplemented!(),
            QSupported(_features) => {
                // send back empty response
                response.flush().map_err(Error::Connection)?;
            }
            Unknown => {
                // send back empty response
                response.flush().map_err(Error::Connection)?;
            }
            Ack => {}
        }

        Ok(())
    }

    fn recv_packet(
        &mut self,
        packet_buffer: &mut Vec<u8>,
        header: u8,
    ) -> Result<(), Error<T::Error, C::Error>> {
        trace!("{:?}", header as char);

        packet_buffer.clear();
        packet_buffer.push(header);

        match header {
            b'$' => {
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
                Ok(())
            }
            _ => Ok(()),
        }
    }

    /// Runs the target
    pub fn run(&mut self, target: &mut T) -> Result<TargetState, Error<T::Error, C::Error>> {
        let mut packet_buffer = Vec::new();
        let mut mem_accesses = Vec::new();

        loop {
            // Handle any incoming GDB commands
            loop {
                let header_byte = if self.paused {
                    // block waiting for a gdb command
                    self.conn.read().map(Some)
                } else {
                    self.conn.read_nonblocking()
                };

                match header_byte {
                    Ok(None) => break, // no incoming message
                    Ok(Some(header_byte)) => {
                        self.recv_packet(&mut packet_buffer, header_byte)?;
                        let command = Command::from_packet(&packet_buffer)
                            .map_err(|e| Error::CommandParse(format!("{:?}", e)))?;
                        self.handle_command(target, command)?;
                    }
                    Err(e) => return Err(Error::Connection(e)),
                };
            }

            // Step the target
            let state = target.step(&mut mem_accesses).map_err(Error::TargetError)?;

            if state == TargetState::Halted {
                return Ok(TargetState::Halted);
            };
        }
    }
}

/// A wrapper around [`Connection`] that computes the single-byte checksum of
/// incoming / outgoing data.
pub struct ResponseWriter<'a, C: 'a> {
    inner: &'a mut C,
    checksum: u8,
    // debug only
    msg: String,
}

impl<'a, C: Connection + 'a> ResponseWriter<'a, C> {
    /// Creates a new ResponseWriter, automatically writing the initial '$'
    pub fn begin(inner: &'a mut C) -> Result<Self, C::Error> {
        inner.write(b'$')?;
        Ok(Self {
            inner,
            checksum: 0,
            msg: "$".to_string(),
        })
    }

    /// Consumes self, automatically writing out the final '#' and the checksum
    pub fn flush(mut self) -> Result<(), C::Error> {
        // don't include '#' in checksum calculation
        let checksum = self.checksum;

        self.write(b'#')?;
        self.write_hex(checksum)?;

        trace!("Reponse: {}", self.msg); // debug only

        Ok(())
    }

    /// Write a single byte.
    pub fn write(&mut self, byte: u8) -> Result<(), C::Error> {
        self.checksum = self.checksum.wrapping_add(byte);
        self.msg.push(byte as char); // debug only
        self.inner.write(byte)
    }

    /// Write an entire buffer over the connection.
    pub fn write_all(&mut self, data: &[u8]) -> Result<(), C::Error> {
        data.iter().try_for_each(|b| self.write(*b))
    }

    /// Write a single byte as a hex string (two ascii chars)
    pub fn write_hex(&mut self, byte: u8) -> Result<(), C::Error> {
        let hex_str = format!("{:02x}", byte);
        self.write(hex_str.as_bytes()[0])?;
        self.write(hex_str.as_bytes()[1])?;
        Ok(())
    }

    /// Write an entire buffer as a hex strings (two ascii chars / byte).
    pub fn write_all_hex(&mut self, data: &[u8]) -> Result<(), C::Error> {
        data.iter().try_for_each(|b| self.write_hex(*b))
    }
}
