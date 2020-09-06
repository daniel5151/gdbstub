use crate::internal::BeBytes;
use crate::protocol::{IdKind, ThreadId};
use crate::Connection;

/// Newtype around a Connection error. Having a newtype allows implementing a
/// `From<ResponseWriterError<C>> for crate::Error<T, C>`, which greatly
/// simplifies some of the error handling in the main gdbstub.
#[derive(Debug, Clone)]
pub struct Error<C>(C);

/// A wrapper around [`Connection`] that computes the single-byte checksum of
/// incoming / outgoing data.
pub struct ResponseWriter<'a, C: Connection + 'a> {
    inner: &'a mut C,
    started: bool,
    checksum: u8,
    // buffer outgoing message, for logging purposes
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
    pub fn flush(mut self) -> Result<(), Error<C::Error>> {
        // don't include '#' in checksum calculation
        let checksum = self.checksum;

        #[cfg(feature = "std")]
        trace!("--> ${}#{:02x?}", self.msg, checksum);

        self.write(b'#')?;
        self.write_hex(checksum)?;

        Ok(())
    }

    /// Get a mutable reference to the underlying connection.
    pub fn as_conn(&mut self) -> &mut C {
        self.inner
    }

    /// Write a single byte.
    pub fn write(&mut self, byte: u8) -> Result<(), Error<C::Error>> {
        #[cfg(feature = "std")]
        self.msg.push(byte as char);

        if !self.started {
            self.started = true;
            self.inner.write(b'$').map_err(Error)?;
        }

        self.checksum = self.checksum.wrapping_add(byte);
        self.inner.write(byte).map_err(Error)
    }

    /// Write an entire buffer over the connection.
    pub fn write_all(&mut self, data: &[u8]) -> Result<(), Error<C::Error>> {
        data.iter().try_for_each(|b| self.write(*b))
    }

    /// Write an entire string over the connection.
    pub fn write_str(&mut self, s: &str) -> Result<(), Error<C::Error>> {
        self.write_all(&s.as_bytes())
    }

    /// Write a single byte as a hex string (two ascii chars)
    fn write_hex(&mut self, byte: u8) -> Result<(), Error<C::Error>> {
        for digit in [(byte & 0xf0) >> 4, byte & 0x0f].iter() {
            let c = match digit {
                0..=9 => b'0' + digit,
                10..=15 => b'a' + digit - 10,
                _ => unreachable!(),
            };
            self.write(c)?;
        }
        Ok(())
    }

    /// Write a byte-buffer as a hex string (i.e: two ascii chars / byte).
    pub fn write_hex_buf(&mut self, data: &[u8]) -> Result<(), Error<C::Error>> {
        data.iter().try_for_each(|b| self.write_hex(*b))
    }

    /// Write data using the binary protocol (i.e: escaping any bytes that are
    /// not 7-bit clean)
    pub fn write_binary(&mut self, data: &[u8]) -> Result<(), Error<C::Error>> {
        data.iter().try_for_each(|b| match b {
            b'#' | b'$' | b'}' | b'*' => {
                self.write(0x7d)?;
                self.write(*b ^ 0x20)
            }
            b if b & 0x80 != 0 => {
                self.write(0x7d)?;
                self.write(*b ^ 0x20)
            }
            _ => self.write(*b),
        })
    }

    /// Write a number as a big-endian hex string using the most compact
    /// representation possible (i.e: trimming leading zeros).
    pub fn write_num<D: BeBytes>(&mut self, digit: D) -> Result<(), Error<C::Error>> {
        let mut buf = [0; 16];
        // infallible (unless digit is a >128 bit number)
        let len = digit.to_be_bytes(&mut buf).unwrap();
        let buf = &buf[..len];
        buf.iter()
            .copied()
            .skip_while(|&b| b == 0)
            .try_for_each(|b| self.write_hex(b))
    }

    pub fn write_id_kind(&mut self, tid: IdKind) -> Result<(), Error<C::Error>> {
        match tid {
            IdKind::All => self.write_str("-1")?,
            IdKind::Any => self.write_str("0")?,
            IdKind::WithID(id) => self.write_num(id.get())?,
        };
        Ok(())
    }

    pub fn write_thread_id(&mut self, tid: ThreadId) -> Result<(), Error<C::Error>> {
        if let Some(pid) = tid.pid {
            self.write_str("p")?;
            self.write_id_kind(pid)?;
            self.write_str(".")?;
        }
        self.write_id_kind(tid.tid)?;
        Ok(())
    }
}
