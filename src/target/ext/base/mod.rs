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

/// Describes the point reached in a replay log for the corresponding stop
/// reason.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReplayLogPosition {
    /// Reached the beginning of the replay log.
    Begin,
    /// Reached the end of the replay log.
    End,
}

/// A handle to check for incoming GDB interrupts.
///
/// At the moment, checking for incoming interrupts requires periodically
/// polling for pending interrupts. e.g:
///
/// ```ignore
/// let interrupts = gdb_interrupt.no_async();
/// loop {
///     if interrupts.pending() {
///         return Ok(StopReason::GdbInterrupt)
///     }
///
///     // execute some number of clock cycles
///     for _ in 0..1024 {
///         match self.system.step() { .. }
///     }
/// }
/// ```
///
/// There is an outstanding issue to add a non-blocking interface to
/// `GdbInterrupt` (see [daniel5151/gdbstub#36](https://github.com/daniel5151/gdbstub/issues/36)).
/// Please comment on the issue if this is something you'd like to see
/// implemented and/or would like to help out with!
pub struct GdbInterrupt<'a> {
    inner: &'a mut dyn FnMut() -> bool,
}

impl<'a> GdbInterrupt<'a> {
    pub(crate) fn new(inner: &'a mut dyn FnMut() -> bool) -> GdbInterrupt<'a> {
        GdbInterrupt { inner }
    }

    /// Returns a [`GdbInterruptNoAsync`] struct which can be polled using a
    /// simple non-blocking [`pending(&mut self) ->
    /// bool`](GdbInterruptNoAsync::pending) method.
    pub fn no_async(self) -> GdbInterruptNoAsync<'a> {
        GdbInterruptNoAsync { inner: self.inner }
    }
}

/// A simplified interface to [`GdbInterrupt`] for projects without
/// async/await infrastructure.
pub struct GdbInterruptNoAsync<'a> {
    inner: &'a mut dyn FnMut() -> bool,
}

impl<'a> GdbInterruptNoAsync<'a> {
    /// Checks if there is a pending GDB interrupt.
    pub fn pending(&mut self) -> bool {
        (self.inner)()
    }
}
