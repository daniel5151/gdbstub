//! Provide exec-file path for the target.
use crate::target::Target;

use crate::common::Pid;

/// Target Extension - Provide current exec-file.
pub trait ExecFile: Target {
    /// Return the full absolute name of the file that was executed to create a
    /// process running on the remote system.
    fn get_exec_file(&self, pid: Option<Pid>) -> &[u8];
}

define_ext!(ExecFileOps, ExecFile);
