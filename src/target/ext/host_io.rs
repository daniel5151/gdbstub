//! Provide Host I/O operations for the target.
use crate::arch::Arch;
use crate::common::{HostMode, HostOpenFlags};
use crate::target::Target;

/// An interface to send pread data back to the GDB client.
pub struct PreadOutput<'a> {
    cb: &'a mut dyn FnMut(&[u8]),
}

impl<'a> PreadOutput<'a> {
    pub(crate) fn new(cb: &'a mut dyn FnMut(&[u8])) -> Self {
        Self { cb }
    }

    /// Write out raw file bytes to the GDB debugger.
    pub fn write(&mut self, buf: &[u8]) {
        (self.cb)(buf)
    }
}

/// Target Extension - Perform I/O operations on host
pub trait HostIo: Target {
    /// Open a file at filename and return a file descriptor for it, or return
    /// -1 if an error occurs.
    fn open(&self, filename: &[u8], flags: HostOpenFlags, mode: HostMode) -> i64;
    /// Close the open file corresponding to fd and return 0, or -1 if an error
    /// occurs.
    fn close(&self, fd: usize) -> i64;
    /// Read data from the open file corresponding to fd.
    fn pread(
        &self,
        fd: usize,
        count: <Self::Arch as Arch>::Usize,
        offset: <Self::Arch as Arch>::Usize,
        output: &mut PreadOutput<'_>,
    ) -> Result<(), Self::Error>;
    /// Select the filesystem on which vFile operations with filename arguments
    /// will operate.
    fn setfs(&self, fd: usize) -> i64;
}

define_ext!(HostIoOps, HostIo);
