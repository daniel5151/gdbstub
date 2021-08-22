use super::prelude::*;
use crate::protocol::commands::ext::ExecFile;

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
                let filename = ops.get_exec_file(cmd.pid);
                res.write_binary_range(filename, cmd.offset, cmd.len)?;
                HandlerStatus::Handled
            }
        };

        Ok(handler_status)
    }
}
