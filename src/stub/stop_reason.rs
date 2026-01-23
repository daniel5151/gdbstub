//! Stop reasons reported back to the GDB client.

use crate::arch::Arch;
use crate::common::Signal;
use crate::common::Tid;
use crate::target::ext::base::reverse_exec::ReplayLogPosition;
use crate::target::ext::breakpoints::WatchKind;
use crate::target::ext::catch_syscalls::CatchSyscallPosition;
use crate::target::Target;

/// Describes why a thread stopped.
///
/// Single threaded targets should set `Tid` to `()`, whereas multi threaded
/// targets should set `Tid` to [`Tid`]. To make things easier, it is
/// recommended to use the [`SingleThreadStopReason`] and
/// [`MultiThreadStopReason`] when possible.
///
///
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
pub enum BaseStopReason<Tid, U> {
    /// Completed the single-step request.
    DoneStep,
    /// The process exited with the specified exit status.
    Exited(u8),
    /// The process terminated with the specified signal number.
    Terminated(Signal),
    /// The program received a signal.
    Signal(Signal),
    /// A specific thread received a signal.
    SignalWithThread {
        /// Tid of the associated thread
        tid: Tid,
        /// The signal
        signal: Signal,
    },
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
        /// Tid of the associated thread
        tid: Tid,
        /// Kind of watchpoint that was hit
        kind: WatchKind,
        /// Address of watched memory
        addr: U,
    },
    /// The program has reached the end of the logged replay events.
    ///
    /// Requires: [`ReverseCont`] or [`ReverseStep`].
    ///
    /// This is used for GDB's reverse execution. When playing back a recording,
    /// you may hit the end of the buffer of recorded events, and as such no
    /// further execution can be done. This stop reason tells GDB that this has
    /// occurred.
    ///
    /// [`ReverseCont`]: crate::target::ext::base::reverse_exec::ReverseCont
    /// [`ReverseStep`]: crate::target::ext::base::reverse_exec::ReverseStep
    ReplayLog {
        /// (optional) Tid of the associated thread.
        tid: Option<Tid>,
        /// The point reached in a replay log (i.e: beginning vs. end).
        pos: ReplayLogPosition,
    },
    /// The program has reached a syscall entry or return location.
    ///
    /// Requires: [`CatchSyscalls`].
    ///
    /// [`CatchSyscalls`]: crate::target::ext::catch_syscalls::CatchSyscalls
    CatchSyscall {
        /// (optional) Tid of the associated thread.
        tid: Option<Tid>,
        /// The syscall number.
        number: U,
        /// The location the event occurred at.
        position: CatchSyscallPosition,
    },
    /// The target's library list has changed.
    ///
    /// This stop reason is used to notify the debugger that the list of loaded
    /// libraries has changed (e.g., a new shared library was loaded). The
    /// debugger can then request the updated library list via
    /// `qXfer:libraries:read` or `qXfer:libraries-svr4:read`.
    ///
    /// Requires: [`Libraries`] or [`LibrariesSvr4`].
    ///
    /// [`Libraries`]: crate::target::ext::libraries::Libraries
    /// [`LibrariesSvr4`]: crate::target::ext::libraries::LibrariesSvr4
    Library(Tid),
}

/// A stop reason for a single threaded target.
///
/// Threads are identified using the unit type `()` (as there is only a single
/// possible thread-id).
pub type SingleThreadStopReason<U> = BaseStopReason<(), U>;

/// A stop reason for a multi threaded target.
///
/// Threads are identified using a [`Tid`].
pub type MultiThreadStopReason<U> = BaseStopReason<Tid, U>;

impl<U> From<BaseStopReason<(), U>> for BaseStopReason<Tid, U> {
    fn from(st_stop_reason: BaseStopReason<(), U>) -> BaseStopReason<Tid, U> {
        match st_stop_reason {
            BaseStopReason::DoneStep => BaseStopReason::DoneStep,
            BaseStopReason::Exited(code) => BaseStopReason::Exited(code),
            BaseStopReason::Terminated(sig) => BaseStopReason::Terminated(sig),
            BaseStopReason::SignalWithThread { signal, .. } => BaseStopReason::SignalWithThread {
                tid: crate::SINGLE_THREAD_TID,
                signal,
            },
            BaseStopReason::SwBreak(_) => BaseStopReason::SwBreak(crate::SINGLE_THREAD_TID),
            BaseStopReason::HwBreak(_) => BaseStopReason::HwBreak(crate::SINGLE_THREAD_TID),
            BaseStopReason::Watch { kind, addr, .. } => BaseStopReason::Watch {
                tid: crate::SINGLE_THREAD_TID,
                kind,
                addr,
            },
            BaseStopReason::Signal(sig) => BaseStopReason::Signal(sig),
            BaseStopReason::ReplayLog { pos, .. } => BaseStopReason::ReplayLog { tid: None, pos },
            BaseStopReason::CatchSyscall {
                number, position, ..
            } => BaseStopReason::CatchSyscall {
                tid: None,
                number,
                position,
            },
            BaseStopReason::Library(_) => BaseStopReason::Library(crate::SINGLE_THREAD_TID),
        }
    }
}

mod private {
    pub trait Sealed {}

    impl<U> Sealed for super::SingleThreadStopReason<U> {}
    impl<U> Sealed for super::MultiThreadStopReason<U> {}
}

/// A marker trait implemented by [`SingleThreadStopReason`] and
/// [`MultiThreadStopReason`].
pub trait IntoStopReason<T: Target>:
    private::Sealed + Into<MultiThreadStopReason<<<T as Target>::Arch as Arch>::Usize>>
{
}

impl<T: Target> IntoStopReason<T> for SingleThreadStopReason<<<T as Target>::Arch as Arch>::Usize> {}
impl<T: Target> IntoStopReason<T> for MultiThreadStopReason<<<T as Target>::Arch as Arch>::Usize> {}
