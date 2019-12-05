use alloc::format;

use crate::Connection;

/// A wrapper around [`Connection`] that computes the single-byte checksum of
/// incoming / outgoing data.
pub struct ResponseWriter<'a, C: 'a> {
    inner: &'a mut C,
    started: bool,
    checksum: u8,
    #[cfg(feature = "std")]
    msg: String,
}

impl<'a, C: Connection + 'a> ResponseWriter<'a, C> {
    /// Creates a new ResponseWriter
    pub fn new(inner: &'a mut C) -> Self {
        Self {
            inner,
            started: false,
            checksum: 0,
            #[cfg(feature = "std")]
            msg: String::new(),
        }
    }

    /// Consumes self, writing out the final '#' and checksum
    pub fn flush(mut self) -> Result<(), C::Error> {
        // don't include '#' in checksum calculation
        let checksum = self.checksum;

        #[cfg(feature = "std")]
        log::trace!("--> ${}#{:02x?}", self.msg, checksum);

        self.write(b'#')?;
        self.write_hex(checksum)?;

        Ok(())
    }

    /// Write a single byte.
    pub fn write(&mut self, byte: u8) -> Result<(), C::Error> {
        #[cfg(feature = "std")]
        self.msg.push(byte as char);

        if !self.started {
            self.started = true;
            self.inner.write(b'$')?;
        }

        self.checksum = self.checksum.wrapping_add(byte);
        self.inner.write(byte)
    }

    /// Write an entire buffer over the connection.
    pub fn write_all(&mut self, data: &[u8]) -> Result<(), C::Error> {
        data.iter().try_for_each(|b| self.write(*b))
    }

    /// Write an entire string over the connection.
    pub fn write_str(&mut self, s: &str) -> Result<(), C::Error> {
        self.write_all(&s.as_bytes())
    }

    /// Write a single byte as a hex string (two ascii chars)
    pub fn write_hex(&mut self, byte: u8) -> Result<(), C::Error> {
        let hex_str = format!("{:02x}", byte);
        self.write(hex_str.as_bytes()[0])?;
        self.write(hex_str.as_bytes()[1])?;
        Ok(())
    }

    /// Write an entire buffer as a hex string (two ascii chars / byte).
    pub fn write_hex_buf(&mut self, data: &[u8]) -> Result<(), C::Error> {
        data.iter().try_for_each(|b| self.write_hex(*b))
    }
}
