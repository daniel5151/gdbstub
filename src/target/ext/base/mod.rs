//! Base operations required to debug any target (read/write memory/registers,
//! step/resume, etc...)
//!
//! It is recommended that single threaded targets implement the simplified
//! `singlethread` API, as `gdbstub` includes optimized implementations of
//! certain internal routines when operating in singlethreaded mode.

pub mod multithread;
pub mod singlethread;

mod single_register_access;

pub use single_register_access::{SingleRegisterAccess, SingleRegisterAccessOps};

/// Base operations for single/multi threaded targets.
pub enum BaseOps<'a, A, E> {
    /// Single-threaded target
    SingleThread(&'a mut dyn singlethread::SingleThreadOps<Arch = A, Error = E>),
    /// Multi-threaded target
    MultiThread(&'a mut dyn multithread::MultiThreadOps<Arch = A, Error = E>),
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
