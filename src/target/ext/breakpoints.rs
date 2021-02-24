//! Add/Remove various kinds of breakpoints.

use crate::arch::Arch;
use crate::target::{Target, TargetResult};

/// Target Extension - Set/Remove Breakpoints.
pub trait Breakpoints: Target {
    /// Set/Remote software breakpoints.
    fn sw_breakpoint(&mut self) -> Option<SwBreakpointOps<Self>> {
        None
    }

    /// Set/Remote hardware breakpoints.
    fn hw_breakpoint(&mut self) -> Option<HwBreakpointOps<Self>> {
        None
    }

    /// Set/Remote hardware watchpoints.
    fn hw_watchpoint(&mut self) -> Option<HwWatchpointOps<Self>> {
        None
    }
}

define_ext!(BreakpointsOps, Breakpoints);

/// Nested Target Extension - Set/Remove Software Breakpoints.
///
/// See [this stackoverflow discussion](https://stackoverflow.com/questions/8878716/what-is-the-difference-between-hardware-and-software-breakpoints)
/// about the differences between hardware and software breakpoints.
///
/// _Recommendation:_ If you're implementing `Target` for an emulator that's
/// using an _interpreted_ CPU (as opposed to a JIT), the simplest way to
/// implement "software" breakpoints would be to check the `PC` value after each
/// CPU cycle, ignoring the specified breakpoint `kind` entirely.
pub trait SwBreakpoint: Target + Breakpoints {
    /// Add a new software breakpoint.
    /// Return `Ok(false)` if the operation could not be completed.
    fn add_sw_breakpoint(
        &mut self,
        addr: <Self::Arch as Arch>::Usize,
        kind: <Self::Arch as Arch>::BreakpointKind,
    ) -> TargetResult<bool, Self>;

    /// Remove an existing software breakpoint.
    /// Return `Ok(false)` if the operation could not be completed.
    fn remove_sw_breakpoint(
        &mut self,
        addr: <Self::Arch as Arch>::Usize,
        kind: <Self::Arch as Arch>::BreakpointKind,
    ) -> TargetResult<bool, Self>;
}

define_ext!(SwBreakpointOps, SwBreakpoint);

/// Nested Target Extension - Set/Remove Hardware Breakpoints.
///
/// See [this stackoverflow discussion](https://stackoverflow.com/questions/8878716/what-is-the-difference-between-hardware-and-software-breakpoints)
/// about the differences between hardware and software breakpoints.
///
/// _Recommendation:_ If you're implementing `Target` for an emulator that's
/// using an _interpreted_ CPU (as opposed to a JIT), there shouldn't be any
/// reason to implement this extension (as software breakpoints are likely to be
/// just-as-fast).
pub trait HwBreakpoint: Target + Breakpoints {
    /// Add a new hardware breakpoint.
    /// Return `Ok(false)` if the operation could not be completed.
    fn add_hw_breakpoint(
        &mut self,
        addr: <Self::Arch as Arch>::Usize,
        kind: <Self::Arch as Arch>::BreakpointKind,
    ) -> TargetResult<bool, Self>;

    /// Remove an existing hardware breakpoint.
    /// Return `Ok(false)` if the operation could not be completed.
    fn remove_hw_breakpoint(
        &mut self,
        addr: <Self::Arch as Arch>::Usize,
        kind: <Self::Arch as Arch>::BreakpointKind,
    ) -> TargetResult<bool, Self>;
}

define_ext!(HwBreakpointOps, HwBreakpoint);

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

/// Nested Target Extension - Set/Remove Hardware Watchpoints.
///
/// See the [GDB documentation](https://sourceware.org/gdb/current/onlinedocs/gdb/Set-Watchpoints.html)
/// regarding watchpoints for how they're supposed to work.
///
/// _Note:_ If this extension isn't implemented, GDB will default to using
/// _software watchpoints_, which tend to be excruciatingly slow (as hey are
/// implemented by single-stepping the system, and reading the watched memory
/// location after each step).
pub trait HwWatchpoint: Target + Breakpoints {
    /// Add a new hardware watchpoint.
    /// Return `Ok(false)` if the operation could not be completed.
    fn add_hw_watchpoint(
        &mut self,
        addr: <Self::Arch as Arch>::Usize,
        kind: WatchKind,
    ) -> TargetResult<bool, Self>;

    /// Remove an existing hardware watchpoint.
    /// Return `Ok(false)` if the operation could not be completed.
    fn remove_hw_watchpoint(
        &mut self,
        addr: <Self::Arch as Arch>::Usize,
        kind: WatchKind,
    ) -> TargetResult<bool, Self>;
}

define_ext!(HwWatchpointOps, HwWatchpoint);
