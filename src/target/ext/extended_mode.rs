//! Enables [Extended Mode](https://sourceware.org/gdb/current/onlinedocs/gdb/Connecting.html)
//! functionality when connecting using `target extended-remote`, such as
//! spawning new processes and/or attaching to existing processes.
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

use crate::common::*;
use crate::target::{Target, TargetResult};

/// Returned from `ExtendedMode::kill`
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

impl ShouldTerminate {
    /// Convert `ShouldTerminate::Yes` into `true`, and `ShouldTerminate::No`
    /// into `false`
    pub fn into_bool(self) -> bool {
        match self {
            ShouldTerminate::Yes => true,
            ShouldTerminate::No => false,
        }
    }
}

/// Describes how the target attached to a process.
pub enum AttachKind {
    /// It attached to an existing process.
    Attach,
    /// It spawned a new process.
    Run,
}

impl AttachKind {
    pub(crate) fn was_attached(self) -> bool {
        match self {
            AttachKind::Attach => true,
            AttachKind::Run => false,
        }
    }
}

/// Target Extension - Support
/// [Extended Mode](https://sourceware.org/gdb/current/onlinedocs/gdb/Connecting.html) functionality.
///
/// # Extended Mode for Single/Multi Threaded Targets
///
/// While extended-mode is primarily intended to be implemented by targets which
/// support debugging multiple processes, there's no reason why a basic
/// single/multi-threaded target can't implement these extensions as well.
///
/// For example, instead of "spawning" a process, the `run` command could be
/// used to reset the execution state instead (e.g: resetting an emulator).
pub trait ExtendedMode: Target {
    /// Spawn and attach to the program `filename`, passing it the provided
    /// `args` on its command line.
    ///
    /// The program is created in the stopped state.
    ///
    /// If no filename is provided, the stub may use a default program (e.g. the
    /// last program run), or a non fatal error should be returned.
    ///
    /// `filename` and `args` are not guaranteed to be valid UTF-8, and are
    /// passed as raw byte arrays. If the filenames/arguments could not be
    /// converted into an appropriate representation, a non fatal error should
    /// be returned.
    ///
    /// _Note:_ This method's implementation should handle any additional
    /// configuration options set via the various `ConfigureXXX` extensions to
    /// `ExtendedMode`. e.g: if the [`ConfigureEnv`](trait.ConfigureEnv.html)
    /// extension is implemented and enabled, this method should set the spawned
    /// processes' environment variables accordingly.
    fn run(&mut self, filename: Option<&[u8]>, args: Args) -> TargetResult<Pid, Self>;

    /// Attach to a new process with the specified PID.
    ///
    /// In all-stop mode, all threads in the attached process are stopped; in
    /// non-stop mode, it may be attached without being stopped (if that is
    /// supported by the target).
    fn attach(&mut self, pid: Pid) -> TargetResult<(), Self>;

    /// Query if specified PID was spawned by the target (via `run`), or if the
    /// target attached to an existing process (via `attach`).
    ///
    /// If the PID doesn't correspond to a process the target has run or
    /// attached to, a non fatal error should be returned.
    fn query_if_attached(&mut self, pid: Pid) -> TargetResult<AttachKind, Self>;

    /// Called when the GDB client sends a Kill request.
    ///
    /// If the PID doesn't correspond to a process the target has run or
    /// attached to, a non fatal error should be returned.
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
    /// The default implementation is a no-op.
    ///
    /// Target implementations can override this implementation if they need to
    /// perform any operations once extended mode is activated.
    fn on_start(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Enable/Disable ASLR for spawned processes.
    fn configure_aslr(&mut self) -> Option<ConfigureAslrOps<Self>> {
        None
    }

    /// Set/Remove/Reset Environment variables for spawned processes.
    fn configure_env(&mut self) -> Option<ConfigureEnvOps<Self>> {
        None
    }

    /// Configure if spawned processes should be spawned using a shell.
    fn configure_startup_shell(&mut self) -> Option<ConfigureStartupShellOps<Self>> {
        None
    }

    /// Configure the working directory for spawned processes.
    fn configure_working_dir(&mut self) -> Option<ConfigureWorkingDirOps<Self>> {
        None
    }
}

define_ext!(ExtendedModeOps, ExtendedMode);

/// Iterator of `args` passed to a spawned process (used in
/// `ExtendedMode::run`)
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

/// Nested Target Extension - Enable/Disable ASLR for spawned processes (for a
/// more consistent debugging experience).
///
/// Corresponds to GDB's [`set disable-randomization`](https://sourceware.org/gdb/onlinedocs/gdb/Starting.html) command.
pub trait ConfigureAslr: ExtendedMode {
    /// Enable/Disable ASLR for spawned processes.
    fn cfg_aslr(&mut self, enabled: bool) -> TargetResult<(), Self>;
}

define_ext!(ConfigureAslrOps, ConfigureAslr);

/// Nested Target Extension - Set/Remove/Reset the Environment variables for
/// spawned processes.
///
/// Corresponds to GDB's [`set environment`](https://sourceware.org/gdb/onlinedocs/gdb/Environment.html#set-environment) cmd.
///
/// _Note:_ Environment variables are not guaranteed to be UTF-8, and are passed
/// as raw byte arrays. If the provided keys/values could not be converted into
/// an appropriate representation, a non fatal error should be returned.
pub trait ConfigureEnv: ExtendedMode {
    /// Set an environment variable.
    fn set_env(&mut self, key: &[u8], val: Option<&[u8]>) -> TargetResult<(), Self>;

    /// Remove an environment variable.
    fn remove_env(&mut self, key: &[u8]) -> TargetResult<(), Self>;

    /// Reset all environment variables to their initial state (i.e: undo all
    /// previous `set/remove_env` calls).
    fn reset_env(&mut self) -> TargetResult<(), Self>;
}

define_ext!(ConfigureEnvOps, ConfigureEnv);

/// Nested Target Extension - Configure if spawned processes should be spawned
/// using a shell.
///
/// Corresponds to GDB's [`set startup-with-shell`](https://sourceware.org/gdb/onlinedocs/gdb/Starting.html) command.
pub trait ConfigureStartupShell: ExtendedMode {
    /// Configure if spawned processes should be spawned using a shell.
    ///
    /// On UNIX-like targets, it is possible to start the inferior using a shell
    /// program. This is the default behavior on both `GDB` and `gdbserver`.
    fn cfg_startup_with_shell(&mut self, enabled: bool) -> TargetResult<(), Self>;
}

define_ext!(ConfigureStartupShellOps, ConfigureStartupShell);

/// Nested Target Extension - Configure the working directory for spawned
/// processes.
///
/// Corresponds to GDB's [`set cwd` and `cd`](https://sourceware.org/gdb/onlinedocs/gdb/Working-Directory.html) commands.
pub trait ConfigureWorkingDir: ExtendedMode {
    /// Set the working directory for spawned processes.
    ///
    /// If no directory is provided, the stub should reset the value to it's
    /// original value.
    ///
    /// The path is not guaranteed to be valid UTF-8, and is passed as a raw
    /// byte array. If the path could not be converted into an appropriate
    /// representation, a non fatal error should be returned.
    fn cfg_working_dir(&mut self, dir: Option<&[u8]>) -> TargetResult<(), Self>;
}

define_ext!(ConfigureWorkingDirOps, ConfigureWorkingDir);
