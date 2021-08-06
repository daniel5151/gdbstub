use super::prelude::*;
use crate::arch::Arch;
use crate::protocol::commands::ext::HostIo;
use crate::target::ext::host_io::{HostStat, PreadOutput};

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

        let handler_status = match command {
            HostIo::vFileOpen(cmd) if ops.enable_open().is_some() => {
                let ops = ops.enable_open().unwrap();
                let ret = ops.open(cmd.filename, cmd.flags, cmd.mode).handle_error()?;
                res.write_str("F")?;
                res.write_num(ret)?;
                HandlerStatus::Handled
            }
            HostIo::vFileClose(cmd) if ops.enable_close().is_some() => {
                let ops = ops.enable_close().unwrap();
                let ret = ops.close(cmd.fd).handle_error()?;
                res.write_str("F")?;
                res.write_num(ret)?;
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
                ops.pread(cmd.fd, count, offset, PreadOutput::new(&mut callback))
                    .handle_error()?;
                err?;

                HandlerStatus::Handled
            }
            HostIo::vFilePwrite(cmd) if ops.enable_pwrite().is_some() => {
                let offset = <T::Arch as Arch>::Usize::from_be_bytes(cmd.offset)
                    .ok_or(Error::TargetMismatch)?;
                let ops = ops.enable_pwrite().unwrap();
                let ret = ops.pwrite(cmd.fd, offset, cmd.data).handle_error()?;
                res.write_str("F")?;
                res.write_num(ret)?;
                HandlerStatus::Handled
            }
            HostIo::vFileFstat(cmd) if ops.enable_fstat().is_some() => {
                let ops = ops.enable_fstat().unwrap();
                let stat = ops.fstat(cmd.fd).handle_error()?;
                let size = core::mem::size_of_val(&stat);
                let p: *const HostStat = &stat;
                let p: *const u8 = p as *const u8;
                res.write_str("F")?;
                res.write_num(size)?;
                res.write_str(";")?;
                res.write_binary(unsafe { core::slice::from_raw_parts(p, size) })?;
                HandlerStatus::Handled
            }
            HostIo::vFileUnlink(cmd) if ops.enable_unlink().is_some() => {
                let ops = ops.enable_unlink().unwrap();
                let ret = ops.unlink(cmd.filename).handle_error()?;
                res.write_str("F")?;
                res.write_num(ret)?;
                HandlerStatus::Handled
            }
            HostIo::vFileReadlink(cmd) if ops.enable_readlink().is_some() => {
                let ops = ops.enable_readlink().unwrap();
                let ret = ops.readlink(cmd.filename).handle_error()?;
                res.write_str("F")?;
                res.write_num(ret)?;
                HandlerStatus::Handled
            }
            HostIo::vFileSetfs(cmd) if ops.enable_setfs().is_some() => {
                let ops = ops.enable_setfs().unwrap();
                ops.setfs(cmd.fs).handle_error()?;
                HandlerStatus::Handled
            }
            _ => HandlerStatus::Handled,
        };

        Ok(handler_status)
    }
}
