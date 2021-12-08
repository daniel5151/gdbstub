#[cfg(feature = "trace-pkt")]
use alloc::string::String;
#[cfg(feature = "trace-pkt")]
use alloc::vec::Vec;

use num_traits::PrimInt;

use crate::conn::Connection;
use crate::internal::BeBytes;
use crate::protocol::{SpecificIdKind, SpecificThreadId};

/// Newtype around a Connection error. Having a newtype allows implementing a
/// `From<ResponseWriterError<C>> for crate::Error<T, C>`, which greatly
/// simplifies some of the error handling in the main gdbstub.
#[derive(Debug, Clone)]
pub struct Error<C>(pub C);

/// A wrapper around [`Connection`] that computes the single-byte checksum of
/// incoming / outgoing data.
pub struct ResponseWriter<'a, C: Connection> {
    inner: &'a mut C,
    started: bool,
    checksum: u8,
    // TODO?: Make using RLE configurable by the target?
    // if implemented correctly, targets that disable RLE entirely could have all RLE code
    // dead-code-eliminated.
    rle_char: u8,
    rle_repeat: u8,
    // buffer to log outgoing packets. only allocates if logging is enabled.
    #[cfg(feature = "trace-pkt")]
    msg: Vec<u8>,
}

impl<'a, C: Connection + 'a> ResponseWriter<'a, C> {
    /// Creates a new ResponseWriter
    pub fn new(inner: &'a mut C) -> Self {
        Self {
            inner,
            started: false,
            checksum: 0,
            rle_char: 0,
            rle_repeat: 0,
            #[cfg(feature = "trace-pkt")]
            msg: Vec::new(),
        }
    }

    /// Consumes self, writing out the final '#' and checksum
    pub fn flush(mut self) -> Result<(), Error<C::Error>> {
        self.write(b'#')?;

        // don't include the '#' in checksum calculation
        // (note: even though `self.write` was called, the the '#' char hasn't been
        // added to the checksum, and is just sitting in the RLE buffer)
        let checksum = self.checksum;

        #[cfg(feature = "trace-pkt")]
        trace!(
            "--> ${}#{:02x?}",
            String::from_utf8_lossy(&self.msg),
            checksum
        );

        self.write_hex(checksum)?;
        // HACK: "write" a dummy char to force an RLE flush
        self.write(0)?;

        self.inner.flush().map_err(Error)?;

        Ok(())
    }

    /// Get a mutable reference to the underlying connection.
    pub fn as_conn(&mut self) -> &mut C {
        self.inner
    }

    fn inner_write(&mut self, byte: u8) -> Result<(), Error<C::Error>> {
        #[cfg(feature = "trace-pkt")]
        if log_enabled!(log::Level::Trace) {
            match self.msg.as_slice() {
                [.., c, b'*'] => {
                    let c = *c;
                    self.msg.pop();
                    for _ in 0..(byte - 29) {
                        self.msg.push(c);
                    }
                }
                _ => self.msg.push(byte),
            }
        }

        if !self.started {
            self.started = true;
            self.inner.write(b'$').map_err(Error)?;
        }

        self.checksum = self.checksum.wrapping_add(byte);
        self.inner.write(byte).map_err(Error)
    }

    fn write(&mut self, byte: u8) -> Result<(), Error<C::Error>> {
        const ASCII_FIRST_PRINT: u8 = b' ';
        const ASCII_LAST_PRINT: u8 = b'~';

        // handle RLE
        let rle_printable = (ASCII_FIRST_PRINT - 4 + (self.rle_repeat + 1)) <= ASCII_LAST_PRINT;
        if byte == self.rle_char && rle_printable {
            self.rle_repeat += 1;
            Ok(())
        } else {
            loop {
                match self.rle_repeat {
                    0 => {} // happens once, after the first char is written
                    // RLE doesn't win, just output the byte
                    1 | 2 | 3 => {
                        for _ in 0..self.rle_repeat {
                            self.inner_write(self.rle_char)?
                        }
                    }
                    // RLE would output an invalid char ('#' or '$')
                    6 | 7 => {
                        self.inner_write(self.rle_char)?;
                        self.rle_repeat -= 1;
                        continue;
                    }
                    // RLE wins for repetitions >4
                    _ => {
                        self.inner_write(self.rle_char)?;
                        self.inner_write(b'*')?;
                        self.inner_write(ASCII_FIRST_PRINT - 4 + self.rle_repeat)?;
                    }
                }

                self.rle_char = byte;
                self.rle_repeat = 1;

                break Ok(());
            }
        }
    }

    /// Write an entire string over the connection.
    pub fn write_str(&mut self, s: &'static str) -> Result<(), Error<C::Error>> {
        for b in s.as_bytes().iter() {
            self.write(*b)?;
        }
        Ok(())
    }

    /// Write a single byte as a hex string (two ascii chars)
    fn write_hex(&mut self, byte: u8) -> Result<(), Error<C::Error>> {
        for &digit in [(byte & 0xf0) >> 4, byte & 0x0f].iter() {
            let c = match digit {
                0..=9 => b'0' + digit,
                10..=15 => b'a' + digit - 10,
                // This match arm is unreachable, but the compiler isn't smart enough to optimize
                // out the branch. As such, using `unreachable!` here would introduce panicking
                // code to `gdbstub`.
                //
                // In this case, it'd be totally reasonable to use
                // `unsafe { core::hint::unreachable_unchecked() }`, but i'll be honest, using some
                // spooky unsafe compiler hints just to eek out a smidge more performance here just
                // isn't worth the cognitive overhead.
                //
                // Moreover, I've played around with this code in godbolt.org, and it turns out that
                // leaving this match arm as `=> digit` ends up generating the _exact same code_ as
                // using `unreachable_unchecked` (at least on x86_64 targets compiled using the
                // latest Rust compiler). YMMV on other platforms.
                _ => digit,
            };
            self.write(c)?;
        }
        Ok(())
    }

    /// Write a byte-buffer as a hex string (i.e: two ascii chars / byte).
    pub fn write_hex_buf(&mut self, data: &[u8]) -> Result<(), Error<C::Error>> {
        for b in data.iter() {
            self.write_hex(*b)?;
        }
        Ok(())
    }

    /// Write data using the binary protocol.
    pub fn write_binary(&mut self, data: &[u8]) -> Result<(), Error<C::Error>> {
        for &b in data.iter() {
            match b {
                b'#' | b'$' | b'}' | b'*' => {
                    self.write(b'}')?;
                    self.write(b ^ 0x20)?
                }
                _ => self.write(b)?,
            }
        }
        Ok(())
    }

    /// Write a number as a big-endian hex string using the most compact
    /// representation possible (i.e: trimming leading zeros).
    pub fn write_num<D: BeBytes + PrimInt>(&mut self, digit: D) -> Result<(), Error<C::Error>> {
        if digit.is_zero() {
            return self.write_hex(0);
        }

        let mut buf = [0; 16];
        // infallible (unless digit is a >128 bit number)
        let len = digit.to_be_bytes(&mut buf).unwrap();
        let buf = &buf[..len];
        for b in buf.iter().copied().skip_while(|&b| b == 0) {
            self.write_hex(b)?
        }
        Ok(())
    }

    fn write_specific_id_kind(&mut self, tid: SpecificIdKind) -> Result<(), Error<C::Error>> {
        match tid {
            SpecificIdKind::All => self.write_str("-1")?,
            SpecificIdKind::WithId(id) => self.write_num(id.get())?,
        };
        Ok(())
    }

    pub fn write_specific_thread_id(
        &mut self,
        tid: SpecificThreadId,
    ) -> Result<(), Error<C::Error>> {
        if let Some(pid) = tid.pid {
            self.write_str("p")?;
            self.write_specific_id_kind(pid)?;
            self.write_str(".")?;
        }
        self.write_specific_id_kind(tid.tid)?;
        Ok(())
    }
}
