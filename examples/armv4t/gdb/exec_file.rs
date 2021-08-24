use gdbstub::common::Pid;
use gdbstub::target;
use gdbstub::target::TargetResult;

use crate::emu::Emu;

impl target::ext::exec_file::ExecFile for Emu {
    fn get_exec_file(&self, _pid: Option<Pid>) -> TargetResult<&[u8], Self> {
        Ok(b"/test.elf")
    }
}
