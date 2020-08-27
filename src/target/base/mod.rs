//! Base operations required to debug any target.

use crate::target::ext::breakpoint::WatchKind;

mod multithread;
mod singlethread;

pub use multithread::MultiThread;
pub use singlethread::SingleThread;

/// Base operations for single/multi threaded targets.
pub enum BaseOps<'a, A, E> {
    /// Single-threaded target
    SingleThread(&'a mut dyn SingleThread<Arch = A, Error = E>),
    /// Multi-threaded target
    MultiThread(&'a mut dyn MultiThread<Arch = A, Error = E>),
}

// It's a common vocabulary type
pub use crate::protocol::TidSelector;

/// Thread ID
// TODO: FUTURE: expose full PID.TID to client?
pub type Tid = core::num::NonZeroUsize;

/// Describes why the target stopped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum StopReason<U> {
    /// Completed the single-step request.
    DoneStep,
    /// `check_gdb_interrupt` returned `true`
    GdbInterrupt,
    /// Halted
    Halted,
    /// Hit a software breakpoint (e.g. due to a trap instruction).
    ///
    /// NOTE: This does not necessarily have to be a breakpoint configured by
    /// the client/user of the current GDB session.
    SwBreak,
    /// Hit a hardware breakpoint.
    HwBreak,
    /// Hit a watchpoint.
    Watch {
        /// Kind of watchpoint that was hit
        kind: WatchKind,
        /// Address of watched memory
        addr: U,
    },
    /// The program received a signal
    Signal(u8),
}

/// Describes how the target should resume the specified thread.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResumeAction {
    /// Continue execution (until the next event occurs).
    Continue,
    /// Step forward a single instruction.
    Step,
    /* ContinueWithSignal(u8),
     * StepWithSignal(u8),
     * Stop,
     * StepInRange(core::ops::Range<U>), */
}

/// An iterator of `(TidSelector, ResumeAction)`, used to specify how particular
/// threads should be resumed. It is _guaranteed_ to contain at least one
/// action.
///
/// See the documentation for
/// [`Target::resume`](trait.Target.html#tymethod.resume) for more details.
pub struct Actions<'a> {
    inner: &'a mut dyn Iterator<Item = (TidSelector, ResumeAction)>,
}

impl Actions<'_> {
    pub(crate) fn new(iter: &mut dyn Iterator<Item = (TidSelector, ResumeAction)>) -> Actions<'_> {
        Actions { inner: iter }
    }
}

impl Iterator for Actions<'_> {
    type Item = (TidSelector, ResumeAction);
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}
