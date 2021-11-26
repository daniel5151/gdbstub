//! Base operations required to debug any target (read/write memory/registers,
//! step/resume, etc...)
//!
//! It is recommended that single threaded targets implement the simplified
//! `singlethread` API, as `gdbstub` includes optimized implementations of
//! certain internal routines when operating in singlethreaded mode.

use crate::arch::Arch;

pub mod multithread;
pub mod singlethread;

mod single_register_access;

pub use single_register_access::{SingleRegisterAccess, SingleRegisterAccessOps};

/// Core required operations for single/multi threaded targets.
pub enum BaseOps<'a, A, E> {
    /// Single-threaded target
    SingleThread(&'a mut dyn singlethread::SingleThreadBase<Arch = A, Error = E>),
    /// Multi-threaded target
    MultiThread(&'a mut dyn multithread::MultiThreadBase<Arch = A, Error = E>),
}

pub(crate) enum ResumeOps<'a, A, E> {
    /// Single-threaded target
    SingleThread(&'a mut dyn singlethread::SingleThreadResume<Arch = A, Error = E>),
    /// Multi-threaded target
    MultiThread(&'a mut dyn multithread::MultiThreadResume<Arch = A, Error = E>),
}

impl<'a, A: Arch, E> BaseOps<'a, A, E> {
    #[inline(always)]
    pub(crate) fn resume_ops(self) -> Option<ResumeOps<'a, A, E>> {
        let ret = match self {
            BaseOps::SingleThread(ops) => ResumeOps::SingleThread(ops.support_resume()?),
            BaseOps::MultiThread(ops) => ResumeOps::MultiThread(ops.support_resume()?),
        };
        Some(ret)
    }
}

/// Describes the point reached in a replay log for the corresponding stop
/// reason.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReplayLogPosition {
    /// Reached the beginning of the replay log.
    Begin,
    /// Reached the end of the replay log.
    End,
}
