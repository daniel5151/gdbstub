//! Base debugging operations for multi threaded targets.

use crate::arch::Arch;
use crate::common::*;
use crate::target::ext::breakpoints::WatchKind;
use crate::target::ext::catch_syscalls::CatchSyscallPosition;
use crate::target::{Target, TargetResult};

use super::{ReplayLogPosition, SingleRegisterAccessOps};

// Convenient re-exports
pub use super::{GdbInterrupt, ResumeAction};

/// Base debugging operations for multi threaded targets.
#[allow(clippy::type_complexity)]
pub trait MultiThreadOps: Target {
    /// Resume execution on the target.
    ///
    /// Prior to calling `resume`, `gdbstub` will call `clear_resume_actions`,
    /// followed by zero or more calls to `set_resume_action`, specifying any
    /// thread-specific resume actions.
    ///
    /// The `default_action` parameter specifies the "fallback" resume action
    /// for any threads that did not have a specific resume action set via
    /// `set_resume_action`. The GDB client typically sets this to
    /// `ResumeAction::Continue`, though this is not guaranteed.
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
    /// breakpoints not working correctly.
    ///
    /// # Additional Considerations
    ///
    /// ### Bare-Metal Targets
    ///
    /// On bare-metal targets (such as microcontrollers or emulators), it's
    /// common to treat individual _CPU cores_ as a separate "threads". e.g:
    /// in a dual-core system, [CPU0, CPU1] might be mapped to [TID1, TID2]
    /// (note that TIDs cannot be zero).
    ///
    /// In this case, the `Tid` argument of `read/write_addrs` becomes quite
    /// relevant, as different cores may have different memory maps.
    ///
    /// ### Running in "Non-stop" mode
    ///
    /// At the moment, `gdbstub` only supports GDB's
    /// ["All-Stop" mode](https://sourceware.org/gdb/current/onlinedocs/gdb/All_002dStop-Mode.html),
    /// whereby _all_ threads must be stopped when returning from `resume`
    /// (not just the thread associated with the `ThreadStopReason`).
    fn resume(
        &mut self,
        default_resume_action: ResumeAction,
        gdb_interrupt: GdbInterrupt<'_>,
    ) -> Result<ThreadStopReason<<Self::Arch as Arch>::Usize>, Self::Error>;

    /// Clear all previously set resume actions.
    fn clear_resume_actions(&mut self) -> Result<(), Self::Error>;

    /// Specify what action each thread should take when
    /// [`resume`](Self::resume) is called.
    ///
    /// A simple implementation of this method would simply update an internal
    /// `HashMap<Tid, ResumeAction>`.
    ///
    /// Aside from the four "base" resume actions handled by this method (i.e:
    /// `Step`, `Continue`, `StepWithSignal`, and `ContinueWithSignal`),
    /// there are also two additional resume actions which are only set if the
    /// target implements their corresponding protocol extension:
    ///
    /// Action                     | Protocol Extension
    /// ---------------------------|---------------------------
    /// Optimized [Range Stepping] | See [`support_range_step()`]
    /// "Stop"                     | Used in "Non-Stop" mode \*
    ///
    /// \* "Non-Stop" mode is currently unimplemented
    ///
    /// [Range Stepping]: https://sourceware.org/gdb/current/onlinedocs/gdb/Continuing-and-Stepping.html#range-stepping
    /// [`support_range_step()`]: Self::support_range_step
    fn set_resume_action(&mut self, tid: Tid, action: ResumeAction) -> Result<(), Self::Error>;

    /// Support for the optimized [range stepping] resume action.
    ///
    /// [range stepping]: https://sourceware.org/gdb/current/onlinedocs/gdb/Continuing-and-Stepping.html#range-stepping
    #[inline(always)]
    fn support_range_step(&mut self) -> Option<MultiThreadRangeSteppingOps<Self>> {
        None
    }

    /// Support for [reverse stepping] a target.
    ///
    /// [reverse stepping]: https://sourceware.org/gdb/current/onlinedocs/gdb/Reverse-Execution.html
    #[inline(always)]
    fn support_reverse_step(&mut self) -> Option<MultiThreadReverseStepOps<Self>> {
        None
    }

    /// Support for [reverse continuing] a target.
    ///
    /// [reverse continuing]: https://sourceware.org/gdb/current/onlinedocs/gdb/Reverse-Execution.html
    #[inline(always)]
    fn support_reverse_cont(&mut self) -> Option<MultiThreadReverseContOps<Self>> {
        None
    }

    /// Read the target's registers.
    ///
    /// If the registers could not be accessed, an appropriate non-fatal error
    /// should be returned.
    fn read_registers(
        &mut self,
        regs: &mut <Self::Arch as Arch>::Registers,
        tid: Tid,
    ) -> TargetResult<(), Self>;

    /// Write the target's registers.
    ///
    /// If the registers could not be accessed, an appropriate non-fatal error
    /// should be returned.
    fn write_registers(
        &mut self,
        regs: &<Self::Arch as Arch>::Registers,
        tid: Tid,
    ) -> TargetResult<(), Self>;

    /// Support for single-register access.
    /// See [`SingleRegisterAccess`](super::SingleRegisterAccess) for more
    /// details.
    ///
    /// While this is an optional feature, it is **highly recommended** to
    /// implement it when possible, as it can significantly improve performance
    /// on certain architectures.
    #[inline(always)]
    fn single_register_access(&mut self) -> Option<SingleRegisterAccessOps<Tid, Self>> {
        None
    }

    /// Read bytes from the specified address range.
    ///
    /// If the requested address range could not be accessed (e.g: due to
    /// MMU protection, unhanded page fault, etc...), an appropriate non-fatal
    /// error should be returned.
    fn read_addrs(
        &mut self,
        start_addr: <Self::Arch as Arch>::Usize,
        data: &mut [u8],
        tid: Tid,
    ) -> TargetResult<(), Self>;

    /// Write bytes to the specified address range.
    ///
    /// If the requested address range could not be accessed (e.g: due to
    /// MMU protection, unhanded page fault, etc...), an appropriate non-fatal
    /// error should be returned.
    fn write_addrs(
        &mut self,
        start_addr: <Self::Arch as Arch>::Usize,
        data: &[u8],
        tid: Tid,
    ) -> TargetResult<(), Self>;

    /// List all currently active threads.
    ///
    /// See [the section above](#bare-metal-targets) on implementing
    /// thread-related methods on bare-metal (threadless) targets.
    fn list_active_threads(
        &mut self,
        thread_is_active: &mut dyn FnMut(Tid),
    ) -> Result<(), Self::Error>;

    /// Check if the specified thread is alive.
    ///
    /// As a convenience, this method provides a default implementation which
    /// uses `list_active_threads` to do a linear-search through all active
    /// threads. On thread-heavy systems, it may be more efficient
    /// to override this method with a more direct query.
    fn is_thread_alive(&mut self, tid: Tid) -> Result<bool, Self::Error> {
        let mut found = false;
        self.list_active_threads(&mut |active_tid| {
            if tid == active_tid {
                found = true;
            }
        })?;
        Ok(found)
    }
}

/// Target Extension - [Reverse continue] for multi threaded targets.
///
/// Reverse continue allows the target to run backwards until it reaches the end
/// of the replay log.
///
/// [Reverse continue]: https://sourceware.org/gdb/current/onlinedocs/gdb/Reverse-Execution.html
pub trait MultiThreadReverseCont: Target + MultiThreadOps {
    /// Reverse-continue the target.
    fn reverse_cont(
        &mut self,
        gdb_interrupt: GdbInterrupt<'_>,
    ) -> Result<ThreadStopReason<<Self::Arch as Arch>::Usize>, Self::Error>;
}

define_ext!(MultiThreadReverseContOps, MultiThreadReverseCont);

/// Target Extension - [Reverse stepping] for multi threaded targets.
///
/// Reverse stepping allows the target to run backwards by one step.
///
/// [Reverse stepping]: https://sourceware.org/gdb/current/onlinedocs/gdb/Reverse-Execution.html
pub trait MultiThreadReverseStep: Target + MultiThreadOps {
    /// Reverse-step the specified [`Tid`].
    fn reverse_step(
        &mut self,
        tid: Tid,
        gdb_interrupt: GdbInterrupt<'_>,
    ) -> Result<ThreadStopReason<<Self::Arch as Arch>::Usize>, Self::Error>;
}

define_ext!(MultiThreadReverseStepOps, MultiThreadReverseStep);

/// Target Extension - Optimized [range stepping] for multi threaded targets.
/// See [`MultiThreadOps::support_range_step`].
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
pub trait MultiThreadRangeStepping: Target + MultiThreadOps {
    /// See [`MultiThreadOps::set_resume_action`].
    fn set_resume_action_range_step(
        &mut self,
        tid: Tid,
        start: <Self::Arch as Arch>::Usize,
        end: <Self::Arch as Arch>::Usize,
    ) -> Result<(), Self::Error>;
}

define_ext!(MultiThreadRangeSteppingOps, MultiThreadRangeStepping);

/// Describes why a thread stopped.
///
/// Targets MUST only respond with stop reasons that correspond to IDETs that
/// target has implemented.
///
/// e.g: A target which has not implemented the [`HwBreakpoint`] IDET must not
/// return a `HwBreak` stop reason. While this is not enforced at compile time,
/// doing so will result in a runtime `UnsupportedStopReason` error.
///
/// [`HwBreakpoint`]: crate::target::ext::breakpoints::HwBreakpoint
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ThreadStopReason<U> {
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
    /// A thread hit a software breakpoint (e.g. due to a trap instruction).
    ///
    /// Requires: [`SwBreakpoint`].
    ///
    /// NOTE: This does not necessarily have to be a breakpoint configured by
    /// the client/user of the current GDB session.
    ///
    /// [`SwBreakpoint`]: crate::target::ext::breakpoints::SwBreakpoint
    SwBreak(Tid),
    /// A thread hit a hardware breakpoint.
    ///
    /// Requires: [`HwBreakpoint`].
    ///
    /// [`HwBreakpoint`]: crate::target::ext::breakpoints::HwBreakpoint
    HwBreak(Tid),
    /// A thread hit a watchpoint.
    ///
    /// Requires: [`HwWatchpoint`].
    ///
    /// [`HwWatchpoint`]: crate::target::ext::breakpoints::HwWatchpoint
    Watch {
        /// Which thread hit the watchpoint
        tid: Tid,
        /// Kind of watchpoint that was hit
        kind: WatchKind,
        /// Address of watched memory
        addr: U,
    },
    /// The program has reached the end of the logged replay events.
    ///
    /// Requires: [`MultiThreadReverseCont`] or [`MultiThreadReverseStep`].
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
