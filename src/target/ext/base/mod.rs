//! Base operations required to debug any target (read/write memory/registers,
//! step/resume, etc...)
//!
//! While not strictly required, it is recommended that single threaded targets
//! implement the simplified `singlethread` API.

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

/// Describes how the target should be resumed.
///
/// Due to a quirk / bug in the mainline GDB client, targets are required to
/// handle the `WithSignal` variants of `Step` and `Continue` regardless of
/// whether or not they have a concept of "signals".
///
/// If your target does not support signals (e.g: the target is a bare-metal
/// microcontroller / emulator), the recommended behavior is to either return a
/// target-specific fatal error, or to handle `{Step,Continue}WithSignal` the
/// same way as their non-`WithSignal` variants.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResumeAction {
    /// Continue execution, stopping once a
    /// [`StopReason`](singlethread::StopReason) occurs.
    Continue,
    /// Step execution.
    Step,
    /// Continue with signal.
    ContinueWithSignal(u8),
    /// Step with signal.
    StepWithSignal(u8),
}
