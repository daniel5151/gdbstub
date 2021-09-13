use gdbstub::common::Pid;
use gdbstub::target;
use gdbstub::target::TargetResult;

use crate::emu::Emu;

impl target::ext::exec_file::ExecFile for Emu {
    fn get_exec_file(
        &self,
        _pid: Option<Pid>,
        offset: usize,
        length: usize,
        buf: &mut [u8],
    ) -> TargetResult<usize, Self> {
        let filename = b"/test.elf";
        let len = filename.len();
        let data = &filename[len.min(offset)..len.min(offset + length)];
        let buf = &mut buf[..data.len()];
        buf.copy_from_slice(data);
        Ok(data.len())
    }
}
