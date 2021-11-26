//! Base debugging operations for single threaded targets.

use crate::arch::Arch;
use crate::common::Signal;
use crate::target::ext::breakpoints::WatchKind;
use crate::target::ext::catch_syscalls::CatchSyscallPosition;
use crate::target::{Target, TargetResult};

use super::{ReplayLogPosition, SingleRegisterAccessOps};

/// Base required debugging operations for single threaded targets.
pub trait SingleThreadBase: Target {
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
    fn support_single_register_access(&mut self) -> Option<SingleRegisterAccessOps<(), Self>> {
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

    /// Support for resuming the target (e.g: via `continue` or `step`)
    #[inline(always)]
    fn support_resume(&mut self) -> Option<SingleThreadResumeOps<Self>> {
        None
    }
}

/// Target extension - support for resuming single threaded targets.
pub trait SingleThreadResume: Target {
    /// Resume execution on the target.
    ///
    /// The GDB client may also include a `signal` which should be passed to the
    /// target.
    ///
    /// # Additional Considerations
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
    fn resume(&mut self, signal: Option<Signal>) -> Result<(), Self::Error>;

    /// Support for optimized [single stepping].
    ///
    /// [single stepping]: https://sourceware.org/gdb/current/onlinedocs/gdb/Continuing-and-Stepping.html#index-stepi
    #[inline(always)]
    fn support_single_step(&mut self) -> Option<SingleThreadSingleStepOps<Self>> {
        None
    }

    /// Support for optimized [range stepping].
    ///
    /// [range stepping]: https://sourceware.org/gdb/current/onlinedocs/gdb/Continuing-and-Stepping.html#range-stepping
    #[inline(always)]
    fn support_range_step(&mut self) -> Option<SingleThreadRangeSteppingOps<Self>> {
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
}

define_ext!(SingleThreadResumeOps, SingleThreadResume);

/// Target Extension - Reverse continue for single threaded targets.
/// See [`SingleThreadResume::support_reverse_cont`].
pub trait SingleThreadReverseCont: Target + SingleThreadResume {
    /// [Reverse continue] the target.
    ///
    /// Reverse continue allows the target to run backwards until it reaches the
    /// end of the replay log.
    ///
    /// [Reverse continue]: https://sourceware.org/gdb/current/onlinedocs/gdb/Reverse-Execution.html
    fn reverse_cont(&mut self) -> Result<(), Self::Error>;
}

define_ext!(SingleThreadReverseContOps, SingleThreadReverseCont);

/// Target Extension - Reverse stepping for single threaded targets.
/// See [`SingleThreadResume::support_reverse_step`].
pub trait SingleThreadReverseStep: Target + SingleThreadResume {
    /// [Reverse step] the target.
    ///
    /// Reverse stepping allows the target to run backwards by one step -
    /// typically a single instruction.
    ///
    /// [Reverse step]: https://sourceware.org/gdb/current/onlinedocs/gdb/Reverse-Execution.html
    fn reverse_step(&mut self) -> Result<(), Self::Error>;
}

define_ext!(SingleThreadReverseStepOps, SingleThreadReverseStep);

/// Target Extension - Optimized [single stepping] for single threaded targets.
/// See [`SingleThreadResume::support_single_step`].
pub trait SingleThreadSingleStep: Target + SingleThreadResume {
    /// [Single step] the target.
    ///
    /// Single stepping will step the target a single "step" - typically a
    /// single instruction.
    /// The GDB client may also include a `signal` which should be passed to the
    /// target.
    ///
    /// [Single step]: https://sourceware.org/gdb/current/onlinedocs/gdb/Continuing-and-Stepping.html#index-stepi
    fn step(&mut self, signal: Option<Signal>) -> Result<(), Self::Error>;
}

define_ext!(SingleThreadSingleStepOps, SingleThreadSingleStep);

/// Target Extension - Optimized range stepping for single threaded targets.
/// See [`SingleThreadResume::support_range_step`].
pub trait SingleThreadRangeStepping: Target + SingleThreadResume {
    /// [Range step] the target.
    ///
    /// Range Stepping will step the target once, and keep stepping the target
    /// as long as execution remains between the specified start (inclusive)
    /// and end (exclusive) addresses, or another stop condition is met
    /// (e.g: a breakpoint it hit).
    ///
    /// If the range is empty (`start` == `end`), then the action becomes
    /// equivalent to the ‘s’ action. In other words, single-step once, and
    /// report the stop (even if the stepped instruction jumps to start).
    ///
    /// _Note:_ A stop reply may be sent at any point even if the PC is still
    /// within the stepping range; for example, it is valid to implement range
    /// stepping in a degenerate way as a single instruction step operation.
    ///
    /// [Range step]: https://sourceware.org/gdb/current/onlinedocs/gdb/Continuing-and-Stepping.html#range-stepping
    fn resume_range_step(
        &mut self,
        start: <Self::Arch as Arch>::Usize,
        end: <Self::Arch as Arch>::Usize,
    ) -> Result<(), Self::Error>;
}

define_ext!(SingleThreadRangeSteppingOps, SingleThreadRangeStepping);

/// Describes why the single-threaded target stopped.
///
/// Targets MUST only respond with stop reasons that correspond to IDETs that
/// target has implemented. Not doing so will result in a runtime error.
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
    /// The process exited with the specified exit status.
    Exited(u8),
    /// The process terminated with the specified signal number.
    Terminated(Signal),
    /// The program received a signal.
    Signal(Signal),
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
}

use crate::target::ext::base::multithread::ThreadStopReason;
use crate::SINGLE_THREAD_TID;

impl<U> From<StopReason<U>> for ThreadStopReason<U> {
    fn from(st_stop_reason: StopReason<U>) -> ThreadStopReason<U> {
        match st_stop_reason {
            StopReason::DoneStep => ThreadStopReason::DoneStep,
            StopReason::Exited(code) => ThreadStopReason::Exited(code),
            StopReason::Terminated(sig) => ThreadStopReason::Terminated(sig),
            StopReason::SwBreak => ThreadStopReason::SwBreak(SINGLE_THREAD_TID),
            StopReason::HwBreak => ThreadStopReason::HwBreak(SINGLE_THREAD_TID),
            StopReason::Watch { kind, addr } => ThreadStopReason::Watch {
                tid: SINGLE_THREAD_TID,
                kind,
                addr,
            },
            StopReason::Signal(sig) => ThreadStopReason::Signal(sig),
            StopReason::ReplayLog(pos) => ThreadStopReason::ReplayLog(pos),
            StopReason::CatchSyscall { number, position } => {
                ThreadStopReason::CatchSyscall { number, position }
            }
        }
    }
}
