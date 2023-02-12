use super::prelude::*;
use crate::protocol::commands::ext::ThreadExtraInfo;
use crate::target::ext::base::BaseOps;

impl<T: Target, C: Connection> GdbStubImpl<T, C> {
    pub(crate) fn handle_thread_extra_info(
        &mut self,
        res: &mut ResponseWriter<'_, C>,
        target: &mut T,
        command: ThreadExtraInfo<'_>,
    ) -> Result<HandlerStatus, Error<T::Error, C::Error>> {
        let ops = match target.base_ops() {
            BaseOps::SingleThread(_) => return Ok(HandlerStatus::Handled),
            BaseOps::MultiThread(ops) => match ops.support_thread_extra_info() {
                Some(ops) => ops,
                None => return Ok(HandlerStatus::Handled),
            },
        };

        crate::__dead_code_marker!("thread_extra_info", "impl");

        let handler_status = match command {
            ThreadExtraInfo::qThreadExtraInfo(info) => {
                let size = ops
                    .thread_extra_info(info.id.tid, info.buf)
                    .map_err(Error::TargetError)?;
                let data = info.buf.get(..size).ok_or(Error::PacketBufferOverflow)?;

                res.write_hex_buf(data)?;

                HandlerStatus::Handled
            }
        };

        Ok(handler_status)
    }
}
