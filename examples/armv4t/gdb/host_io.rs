use gdbstub::target;

use crate::emu::Emu;

use gdbstub::common::{HostMode, HostOpenFlags};
use gdbstub::target::ext::host_io::PreadOutput;

impl target::ext::host_io::HostIo for Emu {
    fn open(&self, filename: &[u8], _flags: HostOpenFlags, _mode: HostMode) -> i64 {
        if filename == b"/proc/1/maps" {
            1
        } else {
            -1
        }
    }

    fn pread(
        &self,
        fd: usize,
        count: u32,
        offset: u32,
        output: &mut PreadOutput<'_>,
    ) -> Result<(), Self::Error> {
        if fd == 1 {
            let maps = b"0x55550000-0x55550078 r-x 0 0 0\n";
            let len = maps.len();
            let count: usize = count as usize;
            let offset: usize = offset as usize;
            output.write(&maps[offset.min(len)..(offset + count).min(len)]);
            Ok(())
        } else {
            Err("pread failed")
        }
    }

    fn close(&self, fd: usize) -> i64 {
        if fd == 1 {
            0
        } else {
            -1
        }
    }

    fn setfs(&self, _fd: usize) -> i64 {
        0
    }
}
