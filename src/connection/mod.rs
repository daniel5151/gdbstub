mod impls;

/// A trait to perform in-order, serial, byte-wise I/O.
///
/// When the `std` feature is enabled, this trait is automatically implemented
/// for [`TcpStream`](std::net::TcpStream) and
/// [`UnixStream`](std::os::unix::net::UnixStream) (on unix systems).
pub trait Connection {
    /// Transport-specific error type.
    type Error;

    /// Write a single byte.
    fn write(&mut self, byte: u8) -> Result<(), Self::Error>;

    /// Write the entire buffer, blocking until complete.
    ///
    /// This method's default implementation calls `self.write()` on each byte
    /// in the buffer. This can be quite inefficient, so if a more efficient
    /// implementation exists (such as calling `write_all()` on an underlying
    /// `std::io::Write` object), this method should be overwritten.
    fn write_all(&mut self, buf: &[u8]) -> Result<(), Self::Error> {
        for b in buf {
            self.write(*b)?;
        }
        Ok(())
    }

    /// Flush this Connection, ensuring that all intermediately buffered
    /// contents reach their destination.
    ///
    /// _Note:_ Not all `Connection`s have internal buffering (e.g: writing data
    /// to a UART TX register with FIFOs disabled). In these cases, it's fine to
    /// simply return `Ok(())`.
    fn flush(&mut self) -> Result<(), Self::Error>;

    /// Peek a single byte. This MUST be a **non-blocking** operation, returning
    /// `None` if no byte is available.
    ///
    /// This is an optional method, as it is only used when polling for GDB
    /// interrupt events as part of a target's `resume` implementation.
    ///
    /// This method's default implementation will always return `None`
    fn peek(&mut self) -> Result<Option<u8>, Self::Error> {
        Ok(None)
    }

    /// Called at the start of a debugging session _before_ any GDB packets have
    /// been sent/received.
    ///
    /// This method's default implementation is a no-op.
    ///
    /// # Example
    ///
    /// The `on_session_start` implementation for `TcpStream` ensures that
    /// [`set_nodelay(true)`](std::net::TcpStream::set_nodelay)
    /// is called. The GDB remote serial protocol requires sending/receiving
    /// many small packets, so forgetting to enable `TCP_NODELAY` can result in
    /// a massively degraded debugging experience.
    fn on_session_start(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

/// Extends [`Connection`] with a blocking `read` method.
///
/// This trait exists as a convenient way to hook into `gdbstub`'s various
/// byte-oriented APIs. It is _entirely optional_, and is not used in any
/// `gdbstub` APIs. The `read` method this trait provides can be reimplemented
/// using direct calls on a concrete type (e.g: using `std::io::Read`).
///
/// When the `std` feature is enabled, this trait is automatically implemented
/// for [`TcpStream`](std::net::TcpStream) and
/// [`UnixStream`](std::os::unix::net::UnixStream) (on unix systems).
pub trait ConnectionExt: Connection {
    /// Read a single byte.
    fn read(&mut self) -> Result<u8, Self::Error>;
}
