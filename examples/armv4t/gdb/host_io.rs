use std::io::{Read, Seek, Write};

use gdbstub::target;
use gdbstub::target::ext::host_io::{
    FsKind, HostIoErrno, HostIoError, HostIoOpenFlags, HostIoOpenMode, HostIoOutput, HostIoResult,
    HostIoStat, HostIoToken,
};

use crate::emu::Emu;

impl target::ext::host_io::HostIo for Emu {
    #[inline(always)]
    fn enable_open(&mut self) -> Option<target::ext::host_io::HostIoOpenOps<Self>> {
        Some(self)
    }

    #[inline(always)]
    fn enable_close(&mut self) -> Option<target::ext::host_io::HostIoCloseOps<Self>> {
        Some(self)
    }

    #[inline(always)]
    fn enable_pread(&mut self) -> Option<target::ext::host_io::HostIoPreadOps<Self>> {
        Some(self)
    }

    #[inline(always)]
    fn enable_pwrite(&mut self) -> Option<target::ext::host_io::HostIoPwriteOps<Self>> {
        Some(self)
    }

    #[inline(always)]
    fn enable_fstat(&mut self) -> Option<target::ext::host_io::HostIoFstatOps<Self>> {
        Some(self)
    }

    #[inline(always)]
    fn enable_unlink(&mut self) -> Option<target::ext::host_io::HostIoUnlinkOps<Self>> {
        Some(self)
    }

    #[inline(always)]
    fn enable_readlink(&mut self) -> Option<target::ext::host_io::HostIoReadlinkOps<Self>> {
        Some(self)
    }

    #[inline(always)]
    fn enable_setfs(&mut self) -> Option<target::ext::host_io::HostIoSetfsOps<Self>> {
        Some(self)
    }
}

impl target::ext::host_io::HostIoOpen for Emu {
    fn open(
        &mut self,
        filename: &[u8],
        flags: HostIoOpenFlags,
        _mode: HostIoOpenMode,
    ) -> HostIoResult<u32, Self> {
        if filename.starts_with(b"/proc") {
            return Err(HostIoError::Errno(HostIoErrno::ENOENT));
        }

        let path =
            std::str::from_utf8(filename).map_err(|_| HostIoError::Errno(HostIoErrno::ENOENT))?;

        let mut read = false;
        let mut write = false;
        if flags.contains(HostIoOpenFlags::O_RDWR) {
            read = true;
            write = true;
        } else if flags.contains(HostIoOpenFlags::O_WRONLY) {
            write = true;
        } else {
            read = true;
        }

        let file = std::fs::OpenOptions::new()
            .read(read)
            .write(write)
            .append(flags.contains(HostIoOpenFlags::O_APPEND))
            .create(flags.contains(HostIoOpenFlags::O_CREAT))
            .truncate(flags.contains(HostIoOpenFlags::O_TRUNC))
            .create_new(flags.contains(HostIoOpenFlags::O_EXCL))
            .open(path)?;

        let n = match self.files.iter_mut().enumerate().find(|(_, f)| f.is_none()) {
            Some((n, free_file)) => {
                *free_file = Some(file);
                n
            }
            None => {
                self.files.push(Some(file));
                self.files.len() - 1
            }
        };

        Ok(n as u32)
    }
}

impl target::ext::host_io::HostIoClose for Emu {
    fn close(&mut self, fd: u32) -> HostIoResult<(), Self> {
        if let Some(file) = self.files.get_mut(fd as usize) {
            file.take().ok_or(HostIoError::Errno(HostIoErrno::EBADF))?;
            while let Some(None) = self.files.last() {
                self.files.pop();
            }
            Ok(())
        } else {
            Err(HostIoError::Errno(HostIoErrno::EBADF))
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
        if let Some(Some(file)) = self.files.get_mut(fd as usize) {
            let mut buffer = vec![0; count as usize];
            file.seek(std::io::SeekFrom::Start(offset as u64))?;
            let n = file.read(&mut buffer)?;
            Ok(output.write(&buffer[..n]))
        } else {
            Err(HostIoError::Errno(HostIoErrno::EBADF))
        }
    }
}

impl target::ext::host_io::HostIoPwrite for Emu {
    fn pwrite(&mut self, fd: u32, offset: u32, data: &[u8]) -> HostIoResult<u32, Self> {
        if let Some(Some(file)) = self.files.get_mut(fd as usize) {
            file.seek(std::io::SeekFrom::Start(offset as u64))?;
            let n = file.write(data)?;
            Ok(n as u32)
        } else {
            Err(HostIoError::Errno(HostIoErrno::EBADF))
        }
    }
}

impl target::ext::host_io::HostIoFstat for Emu {
    fn fstat(&mut self, fd: u32) -> HostIoResult<HostIoStat, Self> {
        if let Some(Some(file)) = self.files.get(fd as usize) {
            let metadata = file.metadata()?;
            macro_rules! time_to_secs {
                ($time:expr) => {
                    $time
                        .map_err(|_| HostIoError::Errno(HostIoErrno::EACCES))?
                        .duration_since(std::time::SystemTime::UNIX_EPOCH)
                        .map_err(|_| HostIoError::Errno(HostIoErrno::EACCES))?
                        .as_secs() as u32
                };
            }
            let atime = time_to_secs!(metadata.accessed());
            let mtime = time_to_secs!(metadata.modified());
            let ctime = time_to_secs!(metadata.created());
            Ok(HostIoStat {
                st_dev: 0,
                st_ino: 0,
                st_mode: HostIoOpenMode::empty(),
                st_nlink: 0,
                st_uid: 0,
                st_gid: 0,
                st_rdev: 0,
                st_size: metadata.len(),
                st_blksize: 0,
                st_blocks: 0,
                st_atime: atime,
                st_mtime: mtime,
                st_ctime: ctime,
            })
        } else {
            Err(HostIoError::Errno(HostIoErrno::EBADF))
        }
    }
}

impl target::ext::host_io::HostIoUnlink for Emu {
    fn unlink(&mut self, filename: &[u8]) -> HostIoResult<(), Self> {
        let path =
            std::str::from_utf8(filename).map_err(|_| HostIoError::Errno(HostIoErrno::ENOENT))?;
        std::fs::remove_file(path)?;
        Ok(())
    }
}

impl target::ext::host_io::HostIoReadlink for Emu {
    fn readlink<'a>(
        &mut self,
        filename: &[u8],
        output: HostIoOutput<'a>,
    ) -> HostIoResult<HostIoToken<'a>, Self> {
        if filename == b"/proc/1/exe" {
            // Support `info proc exe` command
            return Ok(output.write(b"/test.elf"));
        } else if filename == b"/proc/1/cwd" {
            // Support `info proc cwd` command
            return Ok(output.write(b"/"));
        } else if filename.starts_with(b"/proc") {
            return Err(HostIoError::Errno(HostIoErrno::ENOENT));
        }

        let path =
            std::str::from_utf8(filename).map_err(|_| HostIoError::Errno(HostIoErrno::ENOENT))?;
        Ok(output.write(
            std::fs::read_link(path)?
                .to_str()
                .ok_or(HostIoError::Errno(HostIoErrno::ENOENT))?
                .as_bytes(),
        ))
    }
}

impl target::ext::host_io::HostIoSetfs for Emu {
    fn setfs(&mut self, _fs: FsKind) -> HostIoResult<(), Self> {
        Ok(())
    }
}
