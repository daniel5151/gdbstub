//! Enable or disable catching syscalls from the inferior process.

use crate::arch::Arch;
use crate::target::{Target, TargetResult};

/// Target Extension - Enable and disable catching syscalls from the inferior
/// process.
///
/// Implementing this extension allows the target to support the `catch syscall`
/// GDB client command. See [GDB documentation](https://sourceware.org/gdb/onlinedocs/gdb/Set-Catchpoints.html)
/// for further details.
///
/// Corresponds to GDB's [`QCatchSyscalls`](https://sourceware.org/gdb/current/onlinedocs/gdb/General-Query-Packets.html#QCatchSyscalls) command.
pub trait CatchSyscalls: Target {
    /// Enables catching syscalls from the inferior process.
    ///
    /// If `filter` is `None`, then all syscalls should be reported to GDB. If a
    /// filter is provided, only the syscalls listed in the filter should be
    /// reported to GDB.
    ///
    /// Note: filters are not combined, subsequent calls this method should
    /// replace any existing syscall filtering.
    fn enable_catch_syscalls(
        &mut self,
        filter: Option<SyscallNumbers<'_, <Self::Arch as Arch>::Usize>>,
    ) -> TargetResult<(), Self>;

    /// Disables catching syscalls from the inferior process.
    fn disable_catch_syscalls(&mut self) -> TargetResult<(), Self>;
}

define_ext!(CatchSyscallsOps, CatchSyscalls);

/// Describes where the syscall catchpoint was triggered at.
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

impl<U> Iterator for SyscallNumbers<'_, U> {
    type Item = U;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}
