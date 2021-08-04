use super::prelude::*;
use crate::arch::Arch;
use crate::protocol::commands::ext::HostIo;
use crate::target::ext::host_io::PreadOutput;

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
            HostIo::vFileOpen(cmd) => {
                let ret = ops.open(cmd.filename, cmd.flags, cmd.mode);
                res.write_str("F")?;
                res.write_num(ret)?;
                HandlerStatus::Handled
            }
            HostIo::vFileClose(cmd) => {
                let ret = ops.close(cmd.fd);
                res.write_str("F")?;
                res.write_num(ret)?;
                HandlerStatus::Handled
            }
            HostIo::vFilePread(cmd) => {
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

                ops.pread(cmd.fd, count, offset, &mut PreadOutput::new(&mut callback))
                    .map_err(Error::TargetError)?;
                err?;

                HandlerStatus::Handled
            }
            HostIo::vFileSetfs(cmd) => {
                let ret = ops.setfs(cmd.fd);
                res.write_str("F")?;
                res.write_num(ret)?;
                HandlerStatus::Handled
            }
        };

        Ok(handler_status)
    }
}
