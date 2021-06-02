//! Base debugging operations for single threaded targets.

use crate::arch::Arch;
use crate::target::ext::breakpoints::WatchKind;
use crate::target::ext::catch_syscalls::CatchSyscallPosition;
use crate::target::{Target, TargetResult};

use super::{ReplayLogPosition, SingleRegisterAccessOps};

// Convenient re-exports
pub use super::{GdbInterrupt, ResumeAction};

/// Base debugging operations for single threaded targets.
#[allow(clippy::type_complexity)]
pub trait SingleThreadOps: Target {
    /// Resume execution on the target.
    ///
    /// `action` specifies how the target should be resumed (i.e: step or
    /// continue).
    ///
    /// The `check_gdb_interrupt` callback can be invoked to check if GDB sent
    /// an Interrupt packet (i.e: the user pressed Ctrl-C). It's recommended to
    /// invoke this callback every-so-often while the system is running (e.g:
    /// every X cycles/milliseconds). Periodically checking for incoming
    /// interrupt packets is _not_ required, but it is _recommended_.
    ///
    /// # Implementation requirements
    ///
    /// These requirements cannot be satisfied by `gdbstub` internally, and must
    /// be handled on a per-target basis.
    ///
    /// ### Adjusting PC after a breakpoint is hit
    ///
    /// The [GDB remote serial protocol documentation](https://sourceware.org/gdb/current/onlinedocs/gdb/Stop-Reply-Packets.html#swbreak-stop-reason)
    /// notes the following:
    ///
    /// > On some architectures, such as x86, at the architecture level, when a
    /// > breakpoint instruction executes the program counter points at the
    /// > breakpoint address plus an offset. On such targets, the stub is
    /// > responsible for adjusting the PC to point back at the breakpoint
    /// > address.
    ///
    /// Omitting PC adjustment may result in unexpected execution flow and/or
    /// breakpoints not appearing to work correctly.
    fn resume(
        &mut self,
        action: ResumeAction,
        gdb_interrupt: GdbInterrupt<'_>,
    ) -> Result<StopReason<<Self::Arch as Arch>::Usize>, Self::Error>;

    /// Support for the optimized [range stepping] resume action.
    ///
    /// [range stepping]: https://sourceware.org/gdb/current/onlinedocs/gdb/Continuing-and-Stepping.html#range-stepping
    #[inline(always)]
    fn support_resume_range_step(&mut self) -> Option<SingleThreadRangeSteppingOps<Self>> {
        None
    }

    /// Support for [reverse stepping] a target.
    ///
    /// [reverse stepping]: https://sourceware.org/gdb/current/onlinedocs/gdb/Reverse-Execution.html
    #[inline(always)]
    fn support_reverse_step(&mut self) -> Option<SingleThreadReverseStepOps<Self>> {
        None
    }

    /// Support for [reverse continuing] a target.
    ///
    /// [reverse continuing]: https://sourceware.org/gdb/current/onlinedocs/gdb/Reverse-Execution.html
    #[inline(always)]
    fn support_reverse_cont(&mut self) -> Option<SingleThreadReverseContOps<Self>> {
        None
    }

    /// Read the target's registers.
    fn read_registers(
        &mut self,
        regs: &mut <Self::Arch as Arch>::Registers,
    ) -> TargetResult<(), Self>;

    /// Write the target's registers.
    fn write_registers(&mut self, regs: &<Self::Arch as Arch>::Registers)
        -> TargetResult<(), Self>;

    /// Support for single-register access.
    /// See [`SingleRegisterAccess`](super::SingleRegisterAccess) for more
    /// details.
    ///
    /// While this is an optional feature, it is **highly recommended** to
    /// implement it when possible, as it can significantly improve performance
    /// on certain architectures.
    #[inline(always)]
    fn single_register_access(&mut self) -> Option<SingleRegisterAccessOps<(), Self>> {
        None
    }

    /// Read bytes from the specified address range.
    ///
    /// If the requested address range could not be accessed (e.g: due to
    /// MMU protection, unhanded page fault, etc...), an appropriate
    /// non-fatal error should be returned.
    fn read_addrs(
        &mut self,
        start_addr: <Self::Arch as Arch>::Usize,
        data: &mut [u8],
    ) -> TargetResult<(), Self>;

    /// Write bytes to the specified address range.
    ///
    /// If the requested address range could not be accessed (e.g: due to
    /// MMU protection, unhanded page fault, etc...), an appropriate
    /// non-fatal error should be returned.
    fn write_addrs(
        &mut self,
        start_addr: <Self::Arch as Arch>::Usize,
        data: &[u8],
    ) -> TargetResult<(), Self>;
}

/// Target Extension - [Reverse continue] for single threaded targets.
///
/// Reverse continue allows the target to run backwards until it reaches the end
/// of the replay log.
///
/// [Reverse continue]: https://sourceware.org/gdb/current/onlinedocs/gdb/Reverse-Execution.html
pub trait SingleThreadReverseCont: Target + SingleThreadOps {
    /// Reverse-continue the target.
    fn reverse_cont(
        &mut self,
        gdb_interrupt: GdbInterrupt<'_>,
    ) -> Result<StopReason<<Self::Arch as Arch>::Usize>, Self::Error>;
}

define_ext!(SingleThreadReverseContOps, SingleThreadReverseCont);

/// Target Extension - [Reverse stepping] for single threaded targets.
///
/// Reverse stepping allows the target to run backwards by one step.
///
/// [Reverse stepping]: https://sourceware.org/gdb/current/onlinedocs/gdb/Reverse-Execution.html
pub trait SingleThreadReverseStep: Target + SingleThreadOps {
    /// Reverse-step the target.
    fn reverse_step(
        &mut self,
        gdb_interrupt: GdbInterrupt<'_>,
    ) -> Result<StopReason<<Self::Arch as Arch>::Usize>, Self::Error>;
}

define_ext!(SingleThreadReverseStepOps, SingleThreadReverseStep);

/// Target Extension - Optimized [range stepping] for single threaded targets.
/// See [`SingleThreadOps::support_resume_range_step`].
///
/// Range Stepping will step the target once, and keep stepping the target as
/// long as execution remains between the specified start (inclusive) and end
/// (exclusive) addresses, or another stop condition is met (e.g: a breakpoint
/// it hit).
///
/// If the range is empty (`start` == `end`), then the action becomes
/// equivalent to the ‘s’ action. In other words, single-step once, and
/// report the stop (even if the stepped instruction jumps to start).
///
/// _Note:_ A stop reply may be sent at any point even if the PC is still
/// within the stepping range; for example, it is valid to implement range
/// stepping in a degenerate way as a single instruction step operation.
///
/// [range stepping]: https://sourceware.org/gdb/current/onlinedocs/gdb/Continuing-and-Stepping.html#range-stepping
pub trait SingleThreadRangeStepping: Target + SingleThreadOps {
    /// See [`SingleThreadOps::resume`].
    fn resume_range_step(
        &mut self,
        start: <Self::Arch as Arch>::Usize,
        end: <Self::Arch as Arch>::Usize,
        gdb_interrupt: GdbInterrupt<'_>,
    ) -> Result<StopReason<<Self::Arch as Arch>::Usize>, Self::Error>;
}

define_ext!(SingleThreadRangeSteppingOps, SingleThreadRangeStepping);

/// Describes why the target stopped.
///
/// Targets MUST only respond with stop reasons that correspond to IDETs that
/// target has implemented.
///
/// e.g: A target which has not implemented the [`HwBreakpoint`] IDET must not
/// return a `HwBreak` stop reason. While this is not enforced at compile time,
/// doing so will result in a runtime `UnsupportedStopReason` error.
///
/// [`HwBreakpoint`]: crate::target::ext::breakpoints::HwBreakpoint
// NOTE: This is a simplified version of `multithread::ThreadStopReason` that omits any references
// to Tid or threads. Internally, it is converted into multithread::ThreadStopReason.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum StopReason<U> {
    /// Completed the single-step request.
    DoneStep,
    /// `check_gdb_interrupt` returned `true`.
    GdbInterrupt,
    /// The process exited with the specified exit status.
    Exited(u8),
    /// The process terminated with the specified signal number.
    Terminated(u8),
    /// The program received a signal.
    Signal(u8),
    /// Hit a software breakpoint (e.g. due to a trap instruction).
    ///
    /// Requires: [`SwBreakpoint`].
    ///
    /// NOTE: This does not necessarily have to be a breakpoint configured by
    /// the client/user of the current GDB session.
    ///
    /// [`SwBreakpoint`]: crate::target::ext::breakpoints::SwBreakpoint
    SwBreak,
    /// Hit a hardware breakpoint.
    ///
    /// Requires: [`HwBreakpoint`].
    ///
    /// [`HwBreakpoint`]: crate::target::ext::breakpoints::HwBreakpoint
    HwBreak,
    /// Hit a watchpoint.
    ///
    /// Requires: [`HwWatchpoint`].
    ///
    /// [`HwWatchpoint`]: crate::target::ext::breakpoints::HwWatchpoint
    Watch {
        /// Kind of watchpoint that was hit
        kind: WatchKind,
        /// Address of watched memory
        addr: U,
    },
    /// The program has reached the end of the logged replay events.
    ///
    /// Requires: [`SingleThreadReverseCont`] or [`SingleThreadReverseStep`].
    ///
    /// This is used for GDB's reverse execution. When playing back a recording,
    /// you may hit the end of the buffer of recorded events, and as such no
    /// further execution can be done. This stop reason tells GDB that this has
    /// occurred.
    ReplayLog(ReplayLogPosition),
    /// The program has reached a syscall entry or return location.
    ///
    /// Requires: [`CatchSyscalls`].
    ///
    /// [`CatchSyscalls`]: crate::target::ext::catch_syscalls::CatchSyscalls
    CatchSyscall {
        /// The syscall number.
        number: U,
        /// The location the event occured at.
        position: CatchSyscallPosition,
    },
    /// The target has been resumed, and will report a stop reason at some later
    /// point.
    ///
    /// Requires: Using the
    /// [`GdbStubStateMachine`](crate::state_machine::GdbStubStateMachine) API.
    ///
    /// Returning this stop reason will immediately yield control back to
    /// `gdbstub`'s callee, while the target continues to run in the background.
    ///
    /// TODO: more docs
    Defer,
}
