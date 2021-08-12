use super::prelude::*;
use crate::arch::Arch;
use crate::protocol::commands::ext::HostIo;
use crate::target::ext::host_io::{HostIoError, HostIoOutput, HostIoStat};
use crate::GdbStubError;

impl<T: Target, C: Connection> GdbStubImpl<T, C> {
    pub(crate) fn handle_host_io(
        &mut self,
        res: &mut ResponseWriter<C>,
        target: &mut T,
        command: HostIo,
    ) -> Result<HandlerStatus, Error<T::Error, C::Error>> {
        let ops = match target.host_io() {
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
                        res.write_num(errno as i32)?;
                    }
                    Err(HostIoError::Fatal(e)) => return Err(GdbStubError::TargetError(e)),
                }
            }};
        }

        let handler_status = match command {
            HostIo::vFileOpen(cmd) if ops.enable_open().is_some() => {
                let ops = ops.enable_open().unwrap();
                handle_hostio_result! {
                if let Ok(fd) = ops.open(cmd.filename, cmd.flags, cmd.mode) => {
                        res.write_str("F")?;
                        res.write_num(fd)?;
                    }
                }
                HandlerStatus::Handled
            }
            HostIo::vFileClose(cmd) if ops.enable_close().is_some() => {
                let ops = ops.enable_close().unwrap();
                handle_hostio_result! {
                    if let Ok(ret) = ops.close(cmd.fd) => {
                        res.write_str("F")?;
                        res.write_num(ret)?;
                    }
                }
                HandlerStatus::Handled
            }
            HostIo::vFilePread(cmd) if ops.enable_pread().is_some() => {
                let count = <T::Arch as Arch>::Usize::from_be_bytes(cmd.count)
                    .ok_or(Error::TargetMismatch)?;
                let offset = <T::Arch as Arch>::Usize::from_be_bytes(cmd.offset)
                    .ok_or(Error::TargetMismatch)?;
                let mut err: Result<_, Error<T::Error, C::Error>> = Ok(());
                let mut callback = |data: &[u8]| {
                    let e = (|| {
                        res.write_str("F")?;
                        res.write_num(data.len())?;
                        res.write_str(";")?;
                        res.write_binary(data)?;
                        Ok(())
                    })();

                    if let Err(e) = e {
                        err = Err(e)
                    }
                };

                let ops = ops.enable_pread().unwrap();
                handle_hostio_result! {
                    if let Ok(_) = ops.pread(cmd.fd, count, offset, HostIoOutput::new(&mut callback)) => {}
                };
                err?;

                HandlerStatus::Handled
            }
            HostIo::vFilePwrite(cmd) if ops.enable_pwrite().is_some() => {
                let offset = <T::Arch as Arch>::Usize::from_be_bytes(cmd.offset)
                    .ok_or(Error::TargetMismatch)?;
                let ops = ops.enable_pwrite().unwrap();
                handle_hostio_result! {
                    if let Ok(ret) = ops.pwrite(cmd.fd, offset, cmd.data) => {
                        res.write_str("F")?;
                        res.write_num(ret)?;
                    }
                };
                HandlerStatus::Handled
            }
            HostIo::vFileFstat(cmd) if ops.enable_fstat().is_some() => {
                let ops = ops.enable_fstat().unwrap();
                handle_hostio_result! {
                    if let Ok(stat) = ops.fstat(cmd.fd) => {
                        let size = core::mem::size_of::<HostIoStat>();
                        res.write_str("F")?;
                        res.write_num(size)?;
                        res.write_str(";")?;
                        res.write_binary(&stat.st_dev.to_le_bytes())?;
                        res.write_binary(&stat.st_ino.to_le_bytes())?;
                        res.write_binary(&(stat.st_mode.bits()).to_le_bytes())?;
                        res.write_binary(&stat.st_nlink.to_le_bytes())?;
                        res.write_binary(&stat.st_uid.to_le_bytes())?;
                        res.write_binary(&stat.st_gid.to_le_bytes())?;
                        res.write_binary(&stat.st_rdev.to_le_bytes())?;
                        res.write_binary(&stat.st_size.to_le_bytes())?;
                        res.write_binary(&stat.st_blksize.to_le_bytes())?;
                        res.write_binary(&stat.st_blocks.to_le_bytes())?;
                        res.write_binary(&stat.st_atime.to_le_bytes())?;
                        res.write_binary(&stat.st_mtime.to_le_bytes())?;
                        res.write_binary(&stat.st_ctime.to_le_bytes())?;
                    }
                };
                HandlerStatus::Handled
            }
            HostIo::vFileUnlink(cmd) if ops.enable_unlink().is_some() => {
                let ops = ops.enable_unlink().unwrap();
                handle_hostio_result! {
                    if let Ok(ret) = ops.unlink(cmd.filename) => {
                        res.write_str("F")?;
                        res.write_num(ret)?;
                    }
                };
                HandlerStatus::Handled
            }
            HostIo::vFileReadlink(cmd) if ops.enable_readlink().is_some() => {
                let mut err: Result<_, Error<T::Error, C::Error>> = Ok(());
                let mut callback = |data: &[u8]| {
                    let e = (|| {
                        res.write_str("F")?;
                        res.write_num(data.len())?;
                        res.write_str(";")?;
                        res.write_binary(data)?;
                        Ok(())
                    })();

                    if let Err(e) = e {
                        err = Err(e)
                    }
                };

                let ops = ops.enable_readlink().unwrap();
                handle_hostio_result! {
                    if let Ok(_) = ops.readlink(cmd.filename, HostIoOutput::new(&mut callback)) => {}
                };
                err?;

                HandlerStatus::Handled
            }
            HostIo::vFileSetfs(cmd) if ops.enable_setfs().is_some() => {
                let ops = ops.enable_setfs().unwrap();
                handle_hostio_result! {
                    if let Ok(ret) = ops.setfs(cmd.fs) => {
                        res.write_str("F")?;
                        res.write_num(ret)?;
                    }
                };
                HandlerStatus::Handled
            }
            _ => HandlerStatus::Handled,
        };

        Ok(handler_status)
    }
}
