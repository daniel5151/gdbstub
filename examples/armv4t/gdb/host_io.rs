use gdbstub::target;
use std::io::{Read, Seek, Write};

use crate::emu::{Emu, FD_MAX};

use gdbstub::target::ext::host_io::{
    HostIoErrno, HostIoError, HostIoOpenFlags, HostIoOpenMode, HostIoOutput, HostIoResult,
    HostIoStat, HostIoToken,
};

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
    fn enable_unlink(&mut self) -> Option<target::ext::host_io::HostIoUnlinkOps<Self>> {
        Some(self)
    }

    #[inline(always)]
    fn enable_readlink(&mut self) -> Option<target::ext::host_io::HostIoReadlinkOps<Self>> {
        Some(self)
    }
}

impl target::ext::host_io::HostIoOpen for Emu {
    fn open(
        &mut self,
        filename: &[u8],
        flags: HostIoOpenFlags,
        mode: HostIoOpenMode,
    ) -> HostIoResult<u32, Self> {
        // Support `info proc mappings` command
        if filename == b"/proc/1/maps" {
            Ok(FD_MAX + 1)
        } else {
            let path = match std::str::from_utf8(filename) {
                Ok(v) => v,
                Err(_) => return Err(HostIoError::Errno(HostIoErrno::ENOENT)),
            };
            let file;
            if flags
                == HostIoOpenFlags::O_WRONLY | HostIoOpenFlags::O_CREAT | HostIoOpenFlags::O_TRUNC
                && mode
                    == HostIoOpenMode::S_IRUSR | HostIoOpenMode::S_IWUSR | HostIoOpenMode::S_IXUSR
            {
                file = std::fs::File::create(path)?;
            } else if flags == HostIoOpenFlags::O_RDONLY {
                file = std::fs::File::open(path)?;
            } else {
                return Err(HostIoError::Errno(HostIoErrno::EINVAL));
            }
            let n = 0;
            for n in 0..FD_MAX {
                if self.files[n as usize].is_none() {
                    break;
                }
            }
            if n == FD_MAX {
                return Err(HostIoError::Errno(HostIoErrno::ENFILE));
            }
            self.files[n as usize] = Some(file);
            Ok(n)
        }
    }
}

impl target::ext::host_io::HostIoClose for Emu {
    fn close(&mut self, fd: u32) -> HostIoResult<(), Self> {
        if fd == FD_MAX + 1 {
            Ok(())
        } else if fd < FD_MAX {
            self.files[fd as usize]
                .take()
                .ok_or(HostIoError::Errno(HostIoErrno::EBADF))?;
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
        if fd == FD_MAX + 1 {
            let maps = b"0x55550000-0x55550078 r-x 0 0 0\n";
            let len = maps.len();
            let count: usize = count as usize;
            let offset: usize = offset as usize;
            Ok(output.write(&maps[offset.min(len)..(offset + count).min(len)]))
        } else if fd < FD_MAX {
            if let Some(ref mut file) = self.files[fd as usize] {
                let mut buffer = vec![0; count as usize];
                file.seek(std::io::SeekFrom::Start(offset as u64))?;
                let n = file.read(&mut buffer)?;
                Ok(output.write(&buffer[..n]))
            } else {
                Err(HostIoError::Errno(HostIoErrno::EBADF))
            }
        } else {
            Err(HostIoError::Errno(HostIoErrno::EBADF))
        }
    }
}

impl target::ext::host_io::HostIoPwrite for Emu {
    fn pwrite(&mut self, fd: u32, offset: u32, data: &[u8]) -> HostIoResult<u32, Self> {
        if fd < FD_MAX {
            if let Some(ref mut file) = self.files[fd as usize] {
                file.seek(std::io::SeekFrom::Start(offset as u64))?;
                let n = file.write(data)?;
                Ok(n as u32)
            } else {
                Err(HostIoError::Errno(HostIoErrno::EBADF))
            }
        } else {
            Err(HostIoError::Errno(HostIoErrno::EBADF))
        }
    }
}

impl target::ext::host_io::HostIoFstat for Emu {
    fn fstat(&mut self, fd: u32) -> HostIoResult<HostIoStat, Self> {
        if fd < FD_MAX {
            if let Some(ref mut file) = self.files[fd as usize] {
                let metadata = file.metadata()?;
                let mtime = metadata
                    .modified()
                    .map_err(|_| HostIoError::Errno(HostIoErrno::EACCES))?;
                let duration = mtime
                    .duration_since(std::time::SystemTime::UNIX_EPOCH)
                    .map_err(|_| HostIoError::Errno(HostIoErrno::EACCES))?;
                let secs = duration.as_secs() as u32;
                Ok(HostIoStat {
                    st_dev: 0,
                    st_ino: 0,
                    st_mode: HostIoOpenMode::S_IRUSR
                        | HostIoOpenMode::S_IWUSR
                        | HostIoOpenMode::S_IXUSR,
                    st_nlink: 0,
                    st_uid: 0,
                    st_gid: 0,
                    st_rdev: 0,
                    st_size: metadata.len(),
                    st_blksize: 0,
                    st_blocks: 0,
                    st_atime: 0,
                    st_mtime: secs,
                    st_ctime: 0,
                })
            } else {
                Err(HostIoError::Errno(HostIoErrno::EBADF))
            }
        } else {
            Err(HostIoError::Errno(HostIoErrno::EBADF))
        }
    }
}

impl target::ext::host_io::HostIoUnlink for Emu {
    fn unlink(&mut self, filename: &[u8]) -> HostIoResult<(), Self> {
        let path = match std::str::from_utf8(filename) {
            Ok(v) => v,
            Err(_) => return Err(HostIoError::Errno(HostIoErrno::ENOENT)),
        };
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
        }

        let path = match std::str::from_utf8(filename) {
            Ok(v) => v,
            Err(_) => return Err(HostIoError::Errno(HostIoErrno::ENOENT)),
        };
        Ok(output.write(
            std::fs::read_link(path)?
                .to_str()
                .ok_or(HostIoError::Errno(HostIoErrno::ENOENT))?
                .as_bytes(),
        ))
    }
}
