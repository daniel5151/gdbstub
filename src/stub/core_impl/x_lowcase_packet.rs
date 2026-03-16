use super::prelude::*;
use crate::protocol::commands::ext::XLowcasePacket;
use crate::stub::core_impl::base::read_addr_handler;

impl<T: Target, C: Connection> GdbStubImpl<T, C> {
    pub(crate) fn handle_x_lowcase_packet(
        &mut self,
        res: &mut ResponseWriter<'_, C>,
        target: &mut T,
        command: XLowcasePacket<'_>,
    ) -> Result<HandlerStatus, Error<T::Error, C::Error>> {
        if !target.use_x_lowcase_packet() {
            return Ok(HandlerStatus::Handled);
        }

        crate::__dead_code_marker!("x_lowcase_packet", "impl");

        let handler_status = match command {
            XLowcasePacket::x(cmd) => {
                read_addr_handler::<C, T>(
                    |i, data| {
                        // Start data with 'b' to indicate binary data
                        if i == 0 {
                            res.write_str("b")?;
                        }
                        res.write_binary(data)
                    },
                    self.current_mem_tid,
                    target,
                    cmd.buf,
                    cmd.len,
                    cmd.addr,
                )?;

                HandlerStatus::Handled
            }
        };

        Ok(handler_status)
    }
}
