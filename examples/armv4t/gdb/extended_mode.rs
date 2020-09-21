use gdbstub::common::Pid;
use gdbstub::target::TargetResult;
use gdbstub::target_ext;
use gdbstub::target_ext::extended_mode::{
    Args, ExtendedModeBaseOps, RunError, RunResult, ShouldTerminate,
};

use crate::emu::Emu;

/*=====================================
=            Extended Mode            =
=====================================*/

// This is a stub implementation of GDB's Extended Mode extensions.
//
// Truth be told, this particular emulator is _not_ very well suited to running
// in extended mode, as it doesn't technically spawn/attach to any process.
// Nonetheless, it's useful to have a stubbed implementation in-tree which can
// be used for basic usability / regression testing.
//
// If you happen to implement a "proper" extended mode gdbstub, feel free to
// file an issue / open a PR that links to your project!

impl target_ext::extended_mode::ExtendedMode for Emu {
    fn base(&mut self) -> ExtendedModeBaseOps<Self> {
        self
    }
}

impl target_ext::extended_mode::ExtendedModeBase for Emu {
    fn kill(&mut self, pid: Option<Pid>) -> TargetResult<ShouldTerminate, Self> {
        eprintln!("GDB sent a kill request for pid {:?}", pid);
        Ok(ShouldTerminate::No)
    }

    fn restart(&mut self) -> Result<(), Self::Error> {
        eprintln!("GDB sent a restart request");
        Ok(())
    }

    fn attach(&mut self, pid: Pid) -> TargetResult<(), Self> {
        eprintln!("GDB tried to attach to a process with PID {}", pid);
        Err(().into()) // non-specific failure
    }

    fn run(&mut self, filename: Option<&[u8]>, args: Args) -> RunResult<Pid, Self> {
        // simplified example: assume UTF-8 filenames / args
        //
        // To be 100% pedantically correct, consider converting to an `OsStr` in the
        // least lossy way possible (e.g: using the `from_bytes` extension from
        // `std::os::unix::ffi::OsStrExt`).

        let filename = match filename {
            None => None,
            Some(raw) => Some(core::str::from_utf8(raw).map_err(|_| RunError::InvalidFilename)?),
        };
        let args = args
            .map(|raw| core::str::from_utf8(raw).map_err(|_| RunError::InvalidArgs))
            .collect::<Result<Vec<_>, _>>()?;

        eprintln!(
            "GDB tried to run a new process with filename {:?}, and args {:?}",
            filename, args
        );

        Err(().into())
    }
}
