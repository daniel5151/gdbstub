use gdbstub::common::Pid;
use gdbstub::target;

use crate::emu::Emu;

impl target::ext::exec_file::ExecFile for Emu {
    fn get_exec_file(&self, _pid: Option<Pid>) -> &[u8] {
        b"/test.elf"
    }
}
