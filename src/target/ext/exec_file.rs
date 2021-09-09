//! Provide exec-file path for the target.
use crate::target::{Target, TargetResult};

use crate::arch::Arch;
use crate::common::Pid;

/// Target Extension - Provide current exec-file.
///
/// NOTE: this extension is primarily intended to be used alongside the [`Host
/// I/O Extensions`](crate::target::ext::host_io), which enables the GDB client
/// to read the executable file directly from the target
pub trait ExecFile: Target {
    /// Return the full absolute name of the file that was executed to create a
    /// process running on the remote system.
    /// If no `pid` is provided, return the filename corresponding to the
    /// currently executing process.
    fn get_exec_file<'a>(
        &self,
        pid: Option<Pid>,
        offset: <Self::Arch as Arch>::Usize,
        length: <Self::Arch as Arch>::Usize,
        buf: &'a mut [u8],
    ) -> TargetResult<&'a [u8], Self>;
}

define_ext!(ExecFileOps, ExecFile);
