//! Support for various
//! [Extended Mode](https://sourceware.org/gdb/onlinedocs/gdb/Packets.html#extended-mode)
//! features, such as spawning new processes / attaching to existing processes.
//!
//! # Disclaimer
//!
//! While this API has been end-to-end tested and confirmed working with a "toy"
//! target implementation (see the included `armv4t` example), it has _not_ been
//! "battle-tested" with a fully-featured extended-mode capable target.
//!
//! If you end up using this API to implement an extended-mode capable target,
//! _please_ file an issue on the repo detailing any bugs / usability issues you
//! may encountered while implementing this API! If everything happens to Just
//! Work as expected, nonetheless file an issue so that this disclaimer can be
//! removed in future releases!

use crate::target::{Target, TargetError, TargetResult};
use crate::Pid;

/// Returned from `ExtendedModeBase::kill`
///
/// Retuning `ShouldTerminate::Yes` will cause the `GdbStub` to immediately
/// shut down and return a `DisconnectReason::Kill`. Returning
/// `ShouldTerminate::No` will keep the `GdbStub` running and listening for
/// further run/attach requests.
pub enum ShouldTerminate {
    /// Terminate GdbStub
    Yes,
    /// Don't Terminate GdbStub
    No,
}

impl From<ShouldTerminate> for bool {
    fn from(st: ShouldTerminate) -> bool {
        match st {
            ShouldTerminate::Yes => true,
            ShouldTerminate::No => false,
        }
    }
}

/// Describes how the target attached to a process.
#[cfg(not(feature = "alloc"))]
pub enum AttachKind {
    /// It attached to an existing process.
    Attach,
    /// It spawned a new process.
    Run,
}

#[cfg(not(feature = "alloc"))]
impl AttachKind {
    pub(crate) fn was_attached(self) -> bool {
        match self {
            AttachKind::Attach => true,
            AttachKind::Run => false,
        }
    }
}

target_error_wrapper! {
    /// Wrapper around `TargetError` which includes errors specific to the
    /// `ExtendedModeBase::run` method.
    #[non_exhaustive]
    pub enum RunError<E> {
        /// Could not construct valid filename from raw bytes.
        InvalidFilename,
        /// Could not construct valid args from raw bytes.
        InvalidArgs,
        /// Other, more general Target errors.
        TargetError(TargetError<E>),
    }
}

/// Result returned from `ExtendedModeBase::run`
pub type RunResult<T, Tgt> = Result<T, RunError<<Tgt as Target>::Error>>;

/// Target Extension - Support for various
/// [Extended Mode](https://sourceware.org/gdb/onlinedocs/gdb/Packets.html#extended-mode) features.
pub trait ExtendedMode: Target {
    /// Base required extended mode operations
    fn base(&mut self) -> ExtendedModeBaseOps<Self>;
}

/// Base operations required by all extended mode capable targets.
pub trait ExtendedModeBase: ExtendedMode {
    /// Spawn and attach to a new process, returning the process's PID.
    ///
    /// Runs the program `filename`, passing it the provided `args` on its
    /// command line. If no filename is provided, the stub may use a default
    /// program (e.g. the last program run), or return a non-fatal error.
    ///
    /// The program is created in the stopped state.
    ///
    /// Filenames and arguments are passed as raw byte arrays, and are not
    /// guaranteed to be valid UTF-8. If the filenames/arguments could not be
    /// converted into an appropriate representation, return
    /// `Err(RunError::InvalidFilename)` or `Err(RunError::InvalidFilename)`.
    fn run(&mut self, filename: Option<&[u8]>, args: Args) -> RunResult<Pid, Self>;

    /// Attach to a new process with the specified PID.
    ///
    /// In all-stop mode, all threads in the attached process are stopped; in
    /// non-stop mode, it may be attached without being stopped (if that is
    /// supported by the target).
    fn attach(&mut self, pid: Pid) -> TargetResult<(), Self>;

    /// Query if specified PID was spawned by the target (via `run`), or if the
    /// target attached to an existing process (via `attach`).
    ///
    /// This method is only required when the `alloc`/`std` features are
    /// disabled. If `alloc` is available, `gdbstub` will automatically track
    /// this property using a heap-allocated data structure.
    #[cfg(not(feature = "alloc"))]
    fn query_if_attached(&mut self, pid: Pid) -> TargetResult<AttachKind, Self>;

    /// Called when the GDB client sends a Kill request.
    ///
    /// GDB may or may not specify a specific PID to kill. When no PID is
    /// specified, the target is free to decide what to do (e.g: kill the
    /// last-used pid, terminate the connection, etc...).
    ///
    /// If `ShouldTerminate::Yes` is returned, `GdbStub` will immediately stop
    /// and return a `DisconnectReason::Kill`. Otherwise, the connection will
    /// remain open, and `GdbStub` will continue listening for run/attach
    /// requests.
    fn kill(&mut self, pid: Option<Pid>) -> TargetResult<ShouldTerminate, Self>;

    /// Restart the program being debugged.
    ///
    /// The GDB docs don't do a good job describing what a "restart" operation
    /// entails. For reference, the official `gdbserver` seems to kill all
    /// inferior processes, and then re-run whatever program was provided on the
    /// command line (if one was provided).
    ///
    /// _Author's Note:_ Based on my current (as of Sept 2020) understanding of
    /// the GDB client;s source code, it seems that the "R" packet is _never_
    /// sent so-long as the target implements the "vRun" packet (which
    /// corresponds to this trait's `run` method). As such, while `gdbstub`
    /// exposes this functionality, and "requires" an implementation, unless
    /// you're running a fairly old version of GDB, it should be fine to
    /// simply stub it out -- e.g: using the `unimplemented!()` macro /
    /// returning a fatal error.
    fn restart(&mut self) -> Result<(), Self::Error>;

    /// (optional) Invoked when GDB client switches to extended mode.
    ///
    /// Target implementations can override this implementation if they need to
    /// perform any operations once extended mode is activated (e.g: setting a
    /// flag, spawning a process, etc...).
    ///
    /// The default implementation is a no-op.
    fn on_start(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

define_ext!(ExtendedModeBaseOps, ExtendedModeBase);

/// Iterator over a set of `args` for the process to be run.
pub struct Args<'a, 'args> {
    inner: &'a mut dyn Iterator<Item = &'args [u8]>,
}

impl core::fmt::Debug for Args<'_, '_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Args {{ .. }}")
    }
}

impl<'a, 'b> Args<'a, 'b> {
    pub(crate) fn new(inner: &'a mut dyn Iterator<Item = &'b [u8]>) -> Args<'a, 'b> {
        Args { inner }
    }
}

impl<'args> Iterator for Args<'_, 'args> {
    type Item = &'args [u8];

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}
