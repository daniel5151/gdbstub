//! Base debugging operations for multi threaded targets.

use crate::arch::Arch;
use crate::common::Signal;
use crate::common::*;
use crate::target::ext::breakpoints::WatchKind;
use crate::target::ext::catch_syscalls::CatchSyscallPosition;
use crate::target::{Target, TargetResult};

use super::{ReplayLogPosition, SingleRegisterAccessOps};

/// Base debugging operations for multi threaded targets.
pub trait MultiThreadOps: Target {
    /// Resume execution on the target.
    ///
    /// Prior to calling `resume`, `gdbstub` will call `clear_resume_actions`,
    /// followed by zero or more calls to the `set_resume_action_XXX` methods,
    /// specifying any thread-specific resume actions.
    ///
    /// Upon returning from the `resume` method, the target being debugged
    /// should be configured to run according to whatever resume actions the
    /// GDB client had specified using any of the `set_resume_action_XXX`
    /// methods.
    ///
    /// Any thread that wasn't explicitly resumed by a `set_resume_action_XXX`
    /// method should be resumed as though it was resumed with
    /// `set_resume_action_continue`.
    ///
    /// A basic target implementation only needs to implement support for
    /// `set_resume_action_continue`, with all other resume actions requiring
    /// their corresponding protocol extension to be implemented:
    ///
    /// Action                      | Protocol Extension
    /// ----------------------------|------------------------------
    /// Optimized [Single Stepping] | See [`support_single_step()`]
    /// Optimized [Range Stepping]  | See [`support_range_step()`]
    /// "Stop"                      | Used in "Non-Stop" mode \*
    ///
    /// \* "Non-Stop" mode is currently unimplemented in `gdbstub`
    ///
    /// [Single stepping]: https://sourceware.org/gdb/current/onlinedocs/gdb/Continuing-and-Stepping.html#index-stepi
    /// [Range Stepping]: https://sourceware.org/gdb/current/onlinedocs/gdb/Continuing-and-Stepping.html#range-stepping
    /// [`support_single_step()`]: Self::support_single_step
    /// [`support_range_step()`]: Self::support_range_step
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
    fn resume(&mut self) -> Result<(), Self::Error>;

    /// Clear all previously set resume actions.
    fn clear_resume_actions(&mut self) -> Result<(), Self::Error>;

    /// Continue the specified thread.
    ///
    /// See the [`resume`](Self::resume) docs for information on when this is
    /// called.
    ///
    /// The GDB client may also include a `signal` which should be passed to the
    /// target.
    fn set_resume_action_continue(
        &mut self,
        tid: Tid,
        signal: Option<Signal>,
    ) -> Result<(), Self::Error>;

    /// Support for optimized [single stepping].
    ///
    /// [single stepping]: https://sourceware.org/gdb/current/onlinedocs/gdb/Continuing-and-Stepping.html#index-stepi
    #[inline(always)]
    fn support_single_step(&mut self) -> Option<MultiThreadSingleStepOps<Self>> {
        None
    }

    /// Support for optimized [range stepping].
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

/// Target Extension - Reverse continue for multi threaded targets.
/// See [`MultiThreadOps::support_reverse_cont`].
pub trait MultiThreadReverseCont: Target + MultiThreadOps {
    /// [Reverse continue] the target.
    ///
    /// Reverse continue allows the target to run backwards until it reaches the
    /// end of the replay log.
    ///
    /// [Reverse continue]: https://sourceware.org/gdb/current/onlinedocs/gdb/Reverse-Execution.html
    fn reverse_cont(&mut self) -> Result<(), Self::Error>;
}

define_ext!(MultiThreadReverseContOps, MultiThreadReverseCont);

/// Target Extension - Reverse stepping for multi threaded targets.
/// See [`MultiThreadOps::support_reverse_step`].
pub trait MultiThreadReverseStep: Target + MultiThreadOps {
    /// [Reverse step] the specified [`Tid`].
    ///
    /// Reverse stepping allows the target to run backwards by one "step" -
    /// typically a single instruction.
    ///
    /// [Reverse step]: https://sourceware.org/gdb/current/onlinedocs/gdb/Reverse-Execution.html
    fn reverse_step(&mut self, tid: Tid) -> Result<(), Self::Error>;
}

define_ext!(MultiThreadReverseStepOps, MultiThreadReverseStep);

/// Target Extension - Optimized single stepping for multi threaded targets.
/// See [`MultiThreadOps::support_single_step`].
pub trait MultiThreadSingleStep: Target + MultiThreadOps {
    /// [Single step] the specified target thread.
    ///
    /// Single stepping will step the target a single "step" - typically a
    /// single instruction.
    ///
    /// The GDB client may also include a `signal` which should be passed to the
    /// target.
    ///
    /// If your target does not support signals (e.g: the target is a bare-metal
    /// microcontroller / emulator), the recommended behavior is to return a
    /// target-specific fatal error
    ///
    /// [Single step]: https://sourceware.org/gdb/current/onlinedocs/gdb/Continuing-and-Stepping.html#index-stepi
    fn set_resume_action_step(
        &mut self,
        tid: Tid,
        signal: Option<Signal>,
    ) -> Result<(), Self::Error>;
}

define_ext!(MultiThreadSingleStepOps, MultiThreadSingleStep);

/// Target Extension - Optimized range stepping for multi threaded targets.
/// See [`MultiThreadOps::support_range_step`].
pub trait MultiThreadRangeStepping: Target + MultiThreadOps {
    /// [Range step] the specified target thread.
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
/// target has implemented. Not doing so will result in a runtime error.
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
    /// The process exited with the specified exit status.
    Exited(u8),
    /// The process terminated with the specified signal number.
    Terminated(Signal),
    /// The program received a signal.
    Signal(Signal),
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
