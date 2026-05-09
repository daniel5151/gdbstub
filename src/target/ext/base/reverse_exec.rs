//! Support for reverse debugging targets.

use crate::target::Target;

/// Target Extension - Reverse continue for targets.
pub trait ReverseCont: Target {
    /// [Reverse continue] the target.
    ///
    /// Reverse continue allows the target to run backwards until it reaches the
    /// end of the replay log.
    ///
    /// [Reverse continue]: https://sourceware.org/gdb/current/onlinedocs/gdb/Reverse-Execution.html
    fn reverse_cont(&mut self) -> Result<(), Self::Error>;
}

/// See [`ReverseCont`]
pub type ReverseContOps<'a, T> = &'a mut dyn ReverseCont<
    Arch = <T as Target>::Arch,
    Error = <T as Target>::Error,
    Tid = <T as Target>::Tid,
>;

/// Target Extension - Reverse stepping for targets.
pub trait ReverseStep: Target {
    /// [Reverse step] the specified `Tid`.
    ///
    /// On single threaded targets, `thread_id` is set to `()` and can be
    /// ignored.
    ///
    /// Reverse stepping allows the target to run backwards by one "step" -
    /// typically a single instruction.
    ///
    /// [Reverse step]: https://sourceware.org/gdb/current/onlinedocs/gdb/Reverse-Execution.html
    fn reverse_step(&mut self, thread_id: Self::Tid) -> Result<(), Self::Error>;
}

/// See [`ReverseStep`]
pub type ReverseStepOps<'a, T> = &'a mut dyn ReverseStep<
    Arch = <T as Target>::Arch,
    Error = <T as Target>::Error,
    Tid = <T as Target>::Tid,
>;

/// Describes the point reached in a replay log (used in
/// [`StopReasonReporter::replay_log`])
///
/// [`StopReasonReporter::replay_log`]:
///     crate::stub::state_machine::StopReasonReporter::replay_log
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReplayLogPosition {
    /// Reached the beginning of the replay log.
    Begin,
    /// Reached the end of the replay log.
    End,
}
