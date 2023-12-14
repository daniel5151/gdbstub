use super::prelude::*;
use crate::protocol::commands::ext::Libraries;
use crate::protocol::commands::ext::LibrariesSvr4;

impl<T: Target, C: Connection> GdbStubImpl<T, C> {
    pub(crate) fn handle_libraries(
        &mut self,
        res: &mut ResponseWriter<'_, C>,
        target: &mut T,
        command: Libraries<'_>,
    ) -> Result<HandlerStatus, Error<T::Error, C::Error>> {
        let ops = match target.support_libraries() {
            Some(ops) => ops,
            None => return Ok(HandlerStatus::Handled),
        };

        crate::__dead_code_marker!("libraries", "impl");

        let handler_status = match command {
            Libraries::qXferLibrariesRead(cmd) => {
                let ret = ops
                    .get_libraries(cmd.offset, cmd.length, cmd.buf)
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

    pub(crate) fn handle_libraries_svr4(
        &mut self,
        res: &mut ResponseWriter<'_, C>,
        target: &mut T,
        command: LibrariesSvr4<'_>,
    ) -> Result<HandlerStatus, Error<T::Error, C::Error>> {
        let ops = match target.support_libraries_svr4() {
            Some(ops) => ops,
            None => return Ok(HandlerStatus::Handled),
        };

        crate::__dead_code_marker!("libraries", "impl");

        let handler_status = match command {
            LibrariesSvr4::qXferLibrariesSvr4Read(cmd) => {
                let ret = ops
                    .get_libraries_svr4(cmd.offset, cmd.length, cmd.buf)
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
