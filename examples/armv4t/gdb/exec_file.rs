use super::copy_range_to_buf;
use crate::emu::Emu;
use gdbstub::common::Pid;
use gdbstub::target;
use gdbstub::target::TargetResult;

impl target::ext::exec_file::ExecFile for Emu {
    fn get_exec_file(
        &self,
        _pid: Option<Pid>,
        offset: u64,
        length: usize,
        buf: &mut [u8],
    ) -> TargetResult<usize, Self> {
        let filename = b"/test.elf";
        Ok(copy_range_to_buf(filename, offset, length, buf))
    }
}
