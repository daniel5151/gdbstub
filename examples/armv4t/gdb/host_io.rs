use gdbstub::target;

use crate::emu::Emu;

use gdbstub::target::ext::host_io::{
    HostIoErrno, HostIoError, HostIoOpenFlags, HostIoOpenMode, HostIoOutput, HostIoResult,
    HostIoToken,
};

impl target::ext::host_io::HostIo for Emu {
    #[inline(always)]
    fn enable_open(&mut self) -> Option<target::ext::host_io::HostIoOpenOps<Self>> {
        Some(self)
    }

    #[inline(always)]
    fn enable_pread(&mut self) -> Option<target::ext::host_io::HostIoPreadOps<Self>> {
        Some(self)
    }

    #[inline(always)]
    fn enable_close(&mut self) -> Option<target::ext::host_io::HostIoCloseOps<Self>> {
        Some(self)
    }
}

impl target::ext::host_io::HostIoOpen for Emu {
    fn open(
        &mut self,
        filename: &[u8],
        _flags: HostIoOpenFlags,
        _mode: HostIoOpenMode,
    ) -> HostIoResult<u32, Self> {
        // Support `info proc mappings` command
        if filename == b"/proc/1/maps" {
            Ok(1)
        } else {
            Err(HostIoError::Errno(HostIoErrno::EPERM))
        }
    }
}

impl target::ext::host_io::HostIoPread for Emu {
    fn pread<'a>(
        &mut self,
        fd: u32,
        count: u32,
        offset: u32,
        output: HostIoOutput<'a>,
    ) -> HostIoResult<HostIoToken<'a>, Self> {
        if fd == 1 {
            let maps = b"0x55550000-0x55550078 r-x 0 0 0\n";
            let len = maps.len();
            let count: usize = count as usize;
            let offset: usize = offset as usize;
            Ok(output.write(&maps[offset.min(len)..(offset + count).min(len)]))
        } else {
            Err(HostIoError::Errno(HostIoErrno::EPERM))
        }
    }
}

impl target::ext::host_io::HostIoClose for Emu {
    fn close(&mut self, fd: u32) -> HostIoResult<u32, Self> {
        if fd == 1 {
            Ok(0)
        } else {
            Err(HostIoError::Errno(HostIoErrno::EPERM))
        }
    }
}
