//! Provide Host I/O operations for the target.
use crate::target::Target;

/// Target Extension - Perform I/O operations on host
pub trait HostIo: Target {
    /// Open a file at filename and return a file descriptor for it, or return
    /// -1 if an error occurs.
    fn open(&self, filename: &[u8], flags: u64, mode: u64) -> i64;
    /// Close the open file corresponding to fd and return 0, or -1 if an error
    /// occurs.
    fn close(&self, fd: usize) -> i64;
    /// Read data from the open file corresponding to fd.
    fn pread(&self, fd: usize, count: usize, offset: usize) -> &[u8];
    /// Select the filesystem on which vFile operations with filename arguments
    /// will operate.
    fn setfs(&self, fd: usize) -> i64;
}

define_ext!(HostIoOps, HostIo);
