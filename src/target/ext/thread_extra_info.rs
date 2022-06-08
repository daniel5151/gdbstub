//! Provide extra information for a thread
use crate::common::Tid;
use crate::target::Target;

/// Target Extension - Provide extra information for a thread
pub trait ThreadExtraInfo: Target {
    /// Provide extra information about a thread
    ///
    /// GDB queries for extra information for a thread as part of the
    /// `info threads` command.  This function will be called once
    /// for each active thread.
    ///
    /// A string can be copied into `buf` that will then be displayed
    /// to the client.  The string is displayed as `(value)`, such as:
    ///
    /// `Thread 1.1 (value)`
    ///
    /// Return the number of bytes written into `buf`.
    fn thread_extra_info(&self, tid: Tid, buf: &mut [u8]) -> Result<usize, Self::Error>;
}

define_ext!(ThreadExtraInfoOps, ThreadExtraInfo);
