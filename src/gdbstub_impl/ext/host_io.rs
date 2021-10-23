use super::prelude::*;
use crate::protocol::commands::ext::HostIo;

use crate::arch::Arch;
use crate::target::ext::host_io::{HostIoError, HostIoStat};

impl<T: Target, C: Connection> GdbStubImpl<T, C> {
    pub(crate) fn handle_host_io(
        &mut self,
        res: &mut ResponseWriter<C>,
        target: &mut T,
        command: HostIo,
    ) -> Result<HandlerStatus, Error<T::Error, C::Error>> {
        let ops = match target.support_host_io() {
            Some(ops) => ops,
            None => return Ok(HandlerStatus::Handled),
        };

        crate::__dead_code_marker!("host_io", "impl");

        macro_rules! handle_hostio_result {
            ( if let Ok($val:pat) = $ret:expr => $callback:block ) => {{
                match $ret {
                    Ok($val) => $callback,
                    Err(HostIoError::Errno(errno)) => {
                        res.write_str("F-1,")?;
                        res.write_num(errno as u32)?;
                    }
                    Err(HostIoError::Fatal(e)) => return Err(Error::TargetError(e)),
                }
            }};
        }

        let handler_status = match command {
            HostIo::vFileOpen(cmd) if ops.support_open().is_some() => {
                let ops = ops.support_open().unwrap();
                handle_hostio_result! {
                    if let Ok(fd) = ops.open(cmd.filename, cmd.flags, cmd.mode) => {
                        res.write_str("F")?;
                        res.write_num(fd)?;
                    }
                }
                HandlerStatus::Handled
            }
            HostIo::vFileClose(cmd) if ops.support_close().is_some() => {
                let ops = ops.support_close().unwrap();
                handle_hostio_result! {
                    if let Ok(()) = ops.close(cmd.fd) => {
                        res.write_str("F0")?;
                    }
                }
                HandlerStatus::Handled
            }
            HostIo::vFilePread(cmd) if ops.support_pread().is_some() => {
                let ops = ops.support_pread().unwrap();
                handle_hostio_result! {
                    if let Ok(ret) = ops.pread(cmd.fd, cmd.count, cmd.offset, cmd.buf) => {
                        res.write_str("F")?;
                        res.write_num(ret)?;
                        res.write_str(";")?;
                        res.write_binary(cmd.buf.get(..ret).ok_or(Error::PacketBufferOverflow)?)?;
                    }
                };

                HandlerStatus::Handled
            }
            HostIo::vFilePwrite(cmd) if ops.support_pwrite().is_some() => {
                let offset = <T::Arch as Arch>::Usize::from_be_bytes(cmd.offset)
                    .ok_or(Error::TargetMismatch)?;
                let ops = ops.support_pwrite().unwrap();
                handle_hostio_result! {
                    if let Ok(ret) = ops.pwrite(cmd.fd, offset, cmd.data) => {
                        res.write_str("F")?;
                        res.write_num(ret)?;
                    }
                };
                HandlerStatus::Handled
            }
            HostIo::vFileFstat(cmd) if ops.support_fstat().is_some() => {
                let ops = ops.support_fstat().unwrap();
                handle_hostio_result! {
                    if let Ok(stat) = ops.fstat(cmd.fd) => {
                        let size = core::mem::size_of::<HostIoStat>();
                        res.write_str("F")?;
                        res.write_num(size)?;
                        res.write_str(";")?;
                        res.write_binary(&stat.st_dev.to_be_bytes())?;
                        res.write_binary(&stat.st_ino.to_be_bytes())?;
                        res.write_binary(&(stat.st_mode.bits()).to_be_bytes())?;
                        res.write_binary(&stat.st_nlink.to_be_bytes())?;
                        res.write_binary(&stat.st_uid.to_be_bytes())?;
                        res.write_binary(&stat.st_gid.to_be_bytes())?;
                        res.write_binary(&stat.st_rdev.to_be_bytes())?;
                        res.write_binary(&stat.st_size.to_be_bytes())?;
                        res.write_binary(&stat.st_blksize.to_be_bytes())?;
                        res.write_binary(&stat.st_blocks.to_be_bytes())?;
                        res.write_binary(&stat.st_atime.to_be_bytes())?;
                        res.write_binary(&stat.st_mtime.to_be_bytes())?;
                        res.write_binary(&stat.st_ctime.to_be_bytes())?;
                    }
                };
                HandlerStatus::Handled
            }
            HostIo::vFileUnlink(cmd) if ops.support_unlink().is_some() => {
                let ops = ops.support_unlink().unwrap();
                handle_hostio_result! {
                    if let Ok(()) = ops.unlink(cmd.filename) => {
                        res.write_str("F0")?;
                    }
                };
                HandlerStatus::Handled
            }
            HostIo::vFileReadlink(cmd) if ops.support_readlink().is_some() => {
                let ops = ops.support_readlink().unwrap();
                handle_hostio_result! {
                    if let Ok(ret) = ops.readlink(cmd.filename, cmd.buf) => {
                        res.write_str("F")?;
                        res.write_num(ret)?;
                        res.write_str(";")?;
                        res.write_binary(cmd.buf.get(..ret).ok_or(Error::PacketBufferOverflow)?)?;
                    }
                };

                HandlerStatus::Handled
            }
            HostIo::vFileSetfs(cmd) if ops.support_setfs().is_some() => {
                let ops = ops.support_setfs().unwrap();
                handle_hostio_result! {
                    if let Ok(()) = ops.setfs(cmd.fs) => {
                        res.write_str("F0")?;
                    }
                };
                HandlerStatus::Handled
            }
            _ => HandlerStatus::Handled,
        };

        Ok(handler_status)
    }
}
