use super::prelude::*;
use crate::protocol::commands::ext::ExecFile;

impl<T: Target, C: Connection> GdbStubImpl<T, C> {
    pub(crate) fn handle_exec_file(
        &mut self,
        res: &mut ResponseWriter<C>,
        target: &mut T,
        command: ExecFile,
    ) -> Result<HandlerStatus, Error<T::Error, C::Error>> {
        let ops = match target.support_exec_file() {
            Some(ops) => ops,
            None => return Ok(HandlerStatus::Handled),
        };

        crate::__dead_code_marker!("exec_file", "impl");

        let handler_status = match command {
            ExecFile::qXferExecFileRead(cmd) => {
                let ret = ops
                    .get_exec_file(cmd.annex.pid, cmd.offset, cmd.length, cmd.buf)
                    .handle_error()?;
                if ret == 0 {
                    res.write_str("l")?;
                } else {
                    res.write_str("m")?;
                    // TODO: add more specific error variant?
                    res.write_binary(cmd.buf.get(..ret).ok_or(Error::PacketBufferOverflow)?)?;
                }
                HandlerStatus::Handled
            }
        };

        Ok(handler_status)
    }
}
