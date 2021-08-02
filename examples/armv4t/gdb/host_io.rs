use gdbstub::target;

use crate::emu::Emu;

impl target::ext::host_io::HostIo for Emu {
    fn open(&self, filename: &[u8], _flags: u64, _mode: u64) -> i64 {
        if filename == b"/proc/1/maps" {
            1
        } else {
            -1
        }
    }

    fn pread(&self, fd: usize, count: usize, offset: usize) -> &[u8] {
        if fd == 1 {
            let maps = b"0x55550000-0x55550078 r-x 0 0 0\n";
            let len = maps.len();
            &maps[offset.min(len)..(offset + count).min(len)]
        } else {
            b""
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
