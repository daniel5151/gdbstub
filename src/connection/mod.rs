mod impls;

/// A trait to perform bytewise I/O over a serial transport layer.
///
/// When the `std` feature is enabled, this trait is automatically implemented
/// for [`TcpStream`](https://doc.rust-lang.org/std/net/struct.TcpStream.html)
/// and [`UnixStream`](https://doc.rust-lang.org/std/os/unix/net/struct.UnixStream.html)
/// (on unix systems).
pub trait Connection {
    /// Transport-specific error type.
    type Error;

    /// Read a single byte.
    fn read(&mut self) -> Result<u8, Self::Error>;

    /// Read the exact number of bytes required to fill the buffer.
    ///
    /// This method's default implementation calls `self.read()` for each byte
    /// in the buffer. This can be quite inefficient, so if a more efficient
    /// implementation exists (such as calling `read_exact()` on an underlying
    /// std::io::Read object), this method should be overwritten.
    fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), Self::Error> {
        buf.iter_mut().try_for_each(|b| {
            *b = self.read()?;
            Ok(())
        })
    }

    /// Write a single byte.
    fn write(&mut self, byte: u8) -> Result<(), Self::Error>;

    /// Peek a single byte. This should be a **non-blocking** operation
    /// (returning None if no byte is available).
    fn peek(&mut self) -> Result<Option<u8>, Self::Error>;
}
