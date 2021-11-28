//! Stop reasons reported back to the GDB client.

use crate::arch::Arch;
use crate::common::Signal;
use crate::common::Tid;
use crate::target::ext::base::ReplayLogPosition;
use crate::target::ext::breakpoints::WatchKind;
use crate::target::ext::catch_syscalls::CatchSyscallPosition;
use crate::target::Target;

/// Implemented by the singlethread [`StopReason`] and multithread
/// [`ThreadStopReason`].
pub trait IntoStopReason<T: Target>:
    Into<ThreadStopReason<<<T as Target>::Arch as Arch>::Usize>>
{
}

impl<T: Target> IntoStopReason<T> for ThreadStopReason<<<T as Target>::Arch as Arch>::Usize> {}
impl<T: Target> IntoStopReason<T> for StopReason<<<T as Target>::Arch as Arch>::Usize> {}

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

impl<U> From<StopReason<U>> for ThreadStopReason<U> {
    fn from(st_stop_reason: StopReason<U>) -> ThreadStopReason<U> {
        match st_stop_reason {
            StopReason::DoneStep => ThreadStopReason::DoneStep,
            StopReason::Exited(code) => ThreadStopReason::Exited(code),
            StopReason::Terminated(sig) => ThreadStopReason::Terminated(sig),
            StopReason::SwBreak => ThreadStopReason::SwBreak(crate::SINGLE_THREAD_TID),
            StopReason::HwBreak => ThreadStopReason::HwBreak(crate::SINGLE_THREAD_TID),
            StopReason::Watch { kind, addr } => ThreadStopReason::Watch {
                tid: crate::SINGLE_THREAD_TID,
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
