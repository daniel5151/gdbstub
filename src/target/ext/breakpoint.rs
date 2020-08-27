//! Add/Remove various kinds of breakpoints.

use crate::arch::Arch;

use crate::target::Target;

/// The kind of watchpoint that should be set/removed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WatchKind {
    /// Fire when the memory location is written to.
    Write,
    /// Fire when the memory location is read from.
    Read,
    /// Fire when the memory location is written to and/or read from.
    ReadWrite,
}

/// Add / Remove a breakpoint / watchpoint
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BreakOp {
    /// Add a new breakpoint / watchpoint.
    Add,
    /// Remove an existing breakpoint / watchpoint.
    Remove,
}

/// Target Extension - Set/remove Software Breakpoints.
#[allow(clippy::type_complexity)]
pub trait SwBreakpoint: Target {
    /// Set/remove a software breakpoint.
    /// Return `Ok(false)` if the operation could not be completed.
    ///
    /// See [this stackoverflow discussion](https://stackoverflow.com/questions/8878716/what-is-the-difference-between-hardware-and-software-breakpoints)
    /// about the differences between hardware and software breakpoints.
    ///
    /// _Author's recommendation:_ If you're implementing `Target` for an
    /// emulator using an _interpreted_ CPU (as opposed to a JIT), the
    /// simplest way to implement "software" breakpoints is to check the
    /// `PC` value after each CPU cycle.
    fn update_sw_breakpoint(
        &mut self,
        addr: <Self::Arch as Arch>::Usize,
        op: BreakOp,
    ) -> Result<bool, Self::Error>;
}

/// Target Extension - Set/remove Hardware Breakpoints.
pub trait HwBreakpoint: Target + SwBreakpoint {
    /// Set/remove a hardware breakpoint.
    /// Return `Ok(false)` if the operation could not be completed.
    ///
    /// See [this stackoverflow discussion](https://stackoverflow.com/questions/8878716/what-is-the-difference-between-hardware-and-software-breakpoints)
    /// about the differences between hardware and software breakpoints.
    ///
    /// _Author's recommendation:_ If you're implementing `Target` for an
    /// emulator using an _interpreted_ CPU (as opposed to a JIT), there
    /// shouldn't be any reason to implement this method (as software
    /// breakpoints are likely to be just-as-fast).
    fn update_hw_breakpoint(
        &mut self,
        addr: <Self::Arch as Arch>::Usize,
        op: BreakOp,
    ) -> Result<bool, Self::Error>;
}

/// Target Extension - Set/remove Hardware Watchpoints.
pub trait HwWatchpoint: Target + SwBreakpoint {
    /// Set/remove a hardware watchpoint.
    /// Return `Ok(false)` if the operation could not be completed.
    ///
    /// See the [GDB documentation](https://sourceware.org/gdb/current/onlinedocs/gdb/Set-Watchpoints.html)
    /// regarding watchpoints for how they're supposed to work.
    ///
    /// _NOTE:_ If this method isn't implemented, GDB will default to using
    /// _software watchpoints_, which tend to be excruciatingly slow (as
    /// they are implemented by single-stepping the system, and reading the
    /// watched memory location after each step).
    fn update_hw_watchpoint(
        &mut self,
        addr: <Self::Arch as Arch>::Usize,
        op: BreakOp,
        kind: WatchKind,
    ) -> Result<bool, Self::Error>;
}
