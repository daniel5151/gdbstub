use crate::emu::Emu;
use gdbstub::common::Pid;
use gdbstub::target;
use gdbstub::target::ext::extended_mode::Args;
use gdbstub::target::ext::extended_mode::AttachKind;
use gdbstub::target::ext::extended_mode::ShouldTerminate;
use gdbstub::target::TargetResult;

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

impl target::ext::extended_mode::ExtendedMode for Emu {
    fn kill(&mut self, pid: Option<Pid>) -> TargetResult<ShouldTerminate, Self> {
        eprintln!("GDB sent a kill request for pid {:?}", pid);
        Ok(ShouldTerminate::No)
    }

    fn restart(&mut self) -> Result<(), Self::Error> {
        eprintln!("GDB sent a restart request");
        Ok(())
    }

    fn attach(&mut self, pid: Pid) -> TargetResult<(), Self> {
        eprintln!("GDB attached to a process with PID {}", pid);
        // stub implementation: just report the same code, but running under a
        // different pid.
        self.reported_pid = pid;
        Ok(())
    }

    fn run(&mut self, filename: Option<&[u8]>, args: Args<'_, '_>) -> TargetResult<Pid, Self> {
        // simplified example: assume UTF-8 filenames / args
        //
        // To be 100% pedantically correct, consider converting to an `OsStr` in the
        // least lossy way possible (e.g: using the `from_bytes` extension from
        // `std::os::unix::ffi::OsStrExt`).

        let filename = match filename {
            None => None,
            Some(raw) => Some(core::str::from_utf8(raw).map_err(drop)?),
        };
        let args = args
            .map(|raw| core::str::from_utf8(raw).map_err(drop))
            .collect::<Result<Vec<_>, _>>()?;

        eprintln!(
            "GDB tried to run a new process with filename {:?}, and args {:?}",
            filename, args
        );

        self.reset();

        // when running in single-threaded mode, this PID can be anything
        Ok(Pid::new(1337).unwrap())
    }

    fn query_if_attached(&mut self, pid: Pid) -> TargetResult<AttachKind, Self> {
        eprintln!(
            "GDB queried if it was attached to a process with PID {}",
            pid
        );
        Ok(AttachKind::Attach)
    }

    #[inline(always)]
    fn support_configure_aslr(
        &mut self,
    ) -> Option<target::ext::extended_mode::ConfigureAslrOps<'_, Self>> {
        Some(self)
    }

    #[inline(always)]
    fn support_configure_env(
        &mut self,
    ) -> Option<target::ext::extended_mode::ConfigureEnvOps<'_, Self>> {
        Some(self)
    }

    #[inline(always)]
    fn support_configure_startup_shell(
        &mut self,
    ) -> Option<target::ext::extended_mode::ConfigureStartupShellOps<'_, Self>> {
        Some(self)
    }

    #[inline(always)]
    fn support_configure_working_dir(
        &mut self,
    ) -> Option<target::ext::extended_mode::ConfigureWorkingDirOps<'_, Self>> {
        Some(self)
    }

    #[inline(always)]
    fn support_current_active_pid(
        &mut self,
    ) -> Option<target::ext::extended_mode::CurrentActivePidOps<'_, Self>> {
        Some(self)
    }
}

impl target::ext::extended_mode::ConfigureAslr for Emu {
    fn cfg_aslr(&mut self, enabled: bool) -> TargetResult<(), Self> {
        eprintln!("GDB {} ASLR", if enabled { "enabled" } else { "disabled" });
        Ok(())
    }
}

impl target::ext::extended_mode::ConfigureEnv for Emu {
    fn set_env(&mut self, key: &[u8], val: Option<&[u8]>) -> TargetResult<(), Self> {
        // simplified example: assume UTF-8 key/val env vars
        let key = core::str::from_utf8(key).map_err(drop)?;
        let val = match val {
            None => None,
            Some(raw) => Some(core::str::from_utf8(raw).map_err(drop)?),
        };

        eprintln!("GDB tried to set a new env var: {:?}={:?}", key, val);

        Ok(())
    }

    fn remove_env(&mut self, key: &[u8]) -> TargetResult<(), Self> {
        let key = core::str::from_utf8(key).map_err(drop)?;
        eprintln!("GDB tried to set remove a env var: {:?}", key);

        Ok(())
    }

    fn reset_env(&mut self) -> TargetResult<(), Self> {
        eprintln!("GDB tried to reset env vars");

        Ok(())
    }
}

impl target::ext::extended_mode::ConfigureStartupShell for Emu {
    fn cfg_startup_with_shell(&mut self, enabled: bool) -> TargetResult<(), Self> {
        eprintln!(
            "GDB {} startup with shell",
            if enabled { "enabled" } else { "disabled" }
        );
        Ok(())
    }
}

impl target::ext::extended_mode::ConfigureWorkingDir for Emu {
    fn cfg_working_dir(&mut self, dir: Option<&[u8]>) -> TargetResult<(), Self> {
        let dir = match dir {
            None => None,
            Some(raw) => Some(core::str::from_utf8(raw).map_err(drop)?),
        };

        match dir {
            None => eprintln!("GDB reset the working directory"),
            Some(dir) => eprintln!("GDB set the working directory to {:?}", dir),
        }

        Ok(())
    }
}

impl target::ext::extended_mode::CurrentActivePid for Emu {
    fn current_active_pid(&mut self) -> Result<Pid, Self::Error> {
        Ok(self.reported_pid)
    }
}
