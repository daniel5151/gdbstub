use std::net::TcpStream;

use crate::Connection;

impl Connection for TcpStream {
    type Error = std::io::Error;

    fn read(&mut self) -> Result<u8, Self::Error> {
        use std::io::Read;

        self.set_nonblocking(false)?;

        let mut buf = [0u8];
        match Read::read_exact(self, &mut buf) {
            Ok(_) => Ok(buf[0]),
            Err(e) => Err(e),
        }
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), Self::Error> {
        use std::io::Read;

        self.set_nonblocking(false)?;

        Read::read_exact(self, buf)
    }

    fn peek(&mut self) -> Result<Option<u8>, Self::Error> {
        self.set_nonblocking(true)?;

        let mut buf = [0u8];
        match Self::peek(self, &mut buf) {
            Ok(_) => Ok(Some(buf[0])),
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(None),
            Err(e) => Err(e),
        }
    }

    fn write(&mut self, byte: u8) -> Result<(), Self::Error> {
        use std::io::Write;

        Write::write_all(self, &[byte])
    }
}
