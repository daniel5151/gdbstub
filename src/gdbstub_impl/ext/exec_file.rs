use super::prelude::*;
use crate::protocol::commands::ext::ExecFile;

use crate::arch::Arch;

impl<T: Target, C: Connection> GdbStubImpl<T, C> {
    pub(crate) fn handle_exec_file(
        &mut self,
        res: &mut ResponseWriter<C>,
        target: &mut T,
        command: ExecFile,
    ) -> Result<HandlerStatus, Error<T::Error, C::Error>> {
        let ops = match target.exec_file() {
            Some(ops) => ops,
            None => return Ok(HandlerStatus::Handled),
        };

        crate::__dead_code_marker!("exec_file", "impl");

        let handler_status = match command {
            ExecFile::qXferExecFileRead(cmd) => {
                let offset = <T::Arch as Arch>::Usize::from_be_bytes(cmd.offset)
                    .ok_or(Error::TargetMismatch)?;
                let length = <T::Arch as Arch>::Usize::from_be_bytes(cmd.length)
                    .ok_or(Error::TargetMismatch)?;
                let ret = ops
                    .get_exec_file(cmd.pid, offset, length, cmd.buf)
                    .handle_error()?;
                if ret.is_empty() {
                    res.write_str("l")?;
                } else {
                    res.write_str("m")?;
                    res.write_binary(ret)?;
                }
                HandlerStatus::Handled
            }
        };

        Ok(handler_status)
    }
}
