//! Enable or disable catching syscalls from the inferior process.

use crate::arch::Arch;
use crate::target::Target;

/// Target Extension - Enable and disable catching syscalls from the inferior
/// process.
///
/// Corresponds GDB's [`QCatchSyscalls`](https://sourceware.org/gdb/current/onlinedocs/gdb/General-Query-Packets.html#QCatchSyscalls) command.
pub trait CatchSyscalls: Target {
    /// Enables catching syscalls from the inferior process.
    ///
    /// If `filter` not `None`, then only the syscalls listed in the filter
    /// should be reported to GDB.
    ///
    /// Note: filters are not combined, subsequent calls this method should
    /// replace any existing syscall filtering.
    fn enable_catch_syscalls(
        &mut self,
        filter: Option<SyscallNumbers<<Self::Arch as Arch>::Usize>>,
    ) -> Result<(), Self::Error>;

    /// Disables catching syscalls from the inferior process.
    fn disable_catch_syscalls(&mut self) -> Result<(), Self::Error>;
}

define_ext!(CatchSyscallsOps, CatchSyscalls);

/// Describes why a catch syscall event was triggered for the corresponding stop
/// reason.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CatchSyscallPosition {
    /// Reached the entry location of the syscall.
    Entry,
    /// Reached the return location of the syscall.
    Return,
}

/// Iterator of syscall numbers that should be reported to GDB.
pub struct SyscallNumbers<'a, U> {
    pub(crate) inner: &'a mut dyn Iterator<Item = U>,
}

impl<U: num_traits::Unsigned> Iterator for SyscallNumbers<'_, U> {
    type Item = U;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}
