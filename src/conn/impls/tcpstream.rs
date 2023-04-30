use crate::conn::Connection;
use crate::conn::ConnectionExt;
use std::net::TcpStream;

impl Connection for TcpStream {
    type Error = std::io::Error;

    fn write(&mut self, byte: u8) -> Result<(), Self::Error> {
        use std::io::Write;

        Write::write_all(self, &[byte])
    }

    fn write_all(&mut self, buf: &[u8]) -> Result<(), Self::Error> {
        use std::io::Write;

        Write::write_all(self, buf)
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        use std::io::Write;

        Write::flush(self)
    }

    fn on_session_start(&mut self) -> Result<(), Self::Error> {
        // see issue #28
        self.set_nodelay(true)
    }
}

impl ConnectionExt for TcpStream {
    fn read(&mut self) -> Result<u8, Self::Error> {
        use std::io::Read;

        self.set_nonblocking(false)?;

        let mut buf = [0u8];
        match Read::read_exact(self, &mut buf) {
            Ok(_) => Ok(buf[0]),
            Err(e) => Err(e),
        }
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
}
