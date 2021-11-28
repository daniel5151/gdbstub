//! Support for reverse debugging targets.

use crate::target::Target;

/// Target Extension - Reverse continue for targets.
pub trait ReverseCont<Tid>: Target
where
    Tid: crate::is_valid_tid::IsValidTid,
{
    /// [Reverse continue] the target.
    ///
    /// Reverse continue allows the target to run backwards until it reaches the
    /// end of the replay log.
    ///
    /// [Reverse continue]: https://sourceware.org/gdb/current/onlinedocs/gdb/Reverse-Execution.html
    fn reverse_cont(&mut self) -> Result<(), Self::Error>;
}

/// See [`ReverseCont`]
pub type ReverseContOps<'a, Tid, T> =
    &'a mut dyn ReverseCont<Tid, Arch = <T as Target>::Arch, Error = <T as Target>::Error>;

/// Target Extension - Reverse stepping for targets.
pub trait ReverseStep<Tid>: Target
where
    Tid: crate::is_valid_tid::IsValidTid,
{
    /// [Reverse step] the specified `Tid`.
    ///
    /// On single threaded targets, `tid` is set to `()` and can be ignored.
    ///
    /// Reverse stepping allows the target to run backwards by one "step" -
    /// typically a single instruction.
    ///
    /// [Reverse step]: https://sourceware.org/gdb/current/onlinedocs/gdb/Reverse-Execution.html
    fn reverse_step(&mut self, tid: Tid) -> Result<(), Self::Error>;
}

/// See [`ReverseStep`]
pub type ReverseStepOps<'a, Tid, T> =
    &'a mut dyn ReverseStep<Tid, Arch = <T as Target>::Arch, Error = <T as Target>::Error>;

/// Describes the point reached in a replay log (used alongside
/// [`BaseStopReason::ReplayLog`](crate::stub::BaseStopReason::ReplayLog))
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReplayLogPosition {
    /// Reached the beginning of the replay log.
    Begin,
    /// Reached the end of the replay log.
    End,
}
