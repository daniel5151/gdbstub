//! Provide exec-file path for the target.
use crate::target::{Target, TargetResult};

use crate::common::Pid;

/// Target Extension - Provide current exec-file.
///
/// NOTE: this extension is primarily intended to be used alongside the [`Host
/// I/O Extensions`](crate::target::ext::host_io), which enables the GDB client
/// to read the executable file directly from the target
pub trait ExecFile: Target {
    /// Get full absolute name of the file that was executed to create
    /// process `pid` running on the remote system, or the filename
    /// corresponding to the currently executing process if no `pid` is
    /// provided.
    /// Store the name into `buf`, and return the length of name.
    fn get_exec_file(
        &self,
        pid: Option<Pid>,
        offset: usize,
        length: usize,
        buf: &mut [u8],
    ) -> TargetResult<usize, Self>;
}

define_ext!(ExecFileOps, ExecFile);
