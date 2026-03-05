use super::prelude::*;
use crate::protocol::commands::ext::ProcessInfo;

impl<T: Target, C: Connection> GdbStubImpl<T, C> {
    pub(crate) fn handle_process_info(
        &mut self,
        res: &mut ResponseWriter<'_, C>,
        target: &mut T,
        command: ProcessInfo,
    ) -> Result<HandlerStatus, Error<T::Error, C::Error>> {
        let ops = match target.support_process_info() {
            Some(ops) => ops,
            None => return Ok(HandlerStatus::Handled),
        };

        crate::__dead_code_marker!("process_info", "impl");

        let mut write_err = Ok(());
        let mut write_cb = |data: &[u8]| {
            if write_err.is_ok() {
                if let Err(e) = res.write_str(core::str::from_utf8(data).unwrap_or("")) {
                    write_err = Err(e);
                }
            }
        };

        let handler_status = match command {
            ProcessInfo::qHostInfo(_cmd) => {
                ops.host_info(&mut write_cb).map_err(Error::TargetError)?;
                write_err?;
                HandlerStatus::Handled
            }
            ProcessInfo::qProcessInfo(_cmd) => {
                ops.process_info(&mut write_cb)
                    .map_err(Error::TargetError)?;
                write_err?;
                HandlerStatus::Handled
            }
        };

        Ok(handler_status)
    }
}
