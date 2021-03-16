//! Base operations required to debug any target (read/write memory/registers,
//! step/resume, etc...)
//!
//! While not strictly required, it's recommended that single threaded targets
//! implement the simplified `singlethread` API.

pub mod multithread;
pub mod singlethread;

mod description;

pub use description::TargetDescription;
pub use description::TargetDescriptionOps;

/// Base operations for single/multi threaded targets.
pub enum BaseOps<'a, A, E> {
    /// Single-threaded target
    SingleThread(&'a mut dyn singlethread::SingleThreadOps<Arch = A, Error = E>),
    /// Multi-threaded target
    MultiThread(&'a mut dyn multithread::MultiThreadOps<Arch = A, Error = E>),
}

/// Describes how the target should be resumed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResumeAction {
    /// Continue execution (until the next event occurs).
    Continue,
    /// Step forward a single instruction.
    Step,
    /* ContinueWithSignal(u8),
     * StepWithSignal(u8),
     * Stop, // NOTE: won't be relevant until `gdbstub` supports non-stop mode
     * StepInRange(core::ops::Range<U>), */
}
