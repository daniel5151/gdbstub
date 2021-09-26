use super::prelude::*;
use crate::protocol::commands::ext::XUpcasePacket;

use crate::arch::Arch;
use crate::target::ext::base::BaseOps;

impl<T: Target, C: Connection> GdbStubImpl<T, C> {
    pub(crate) fn handle_x_upcase_packet<'a>(
        &mut self,
        _res: &mut ResponseWriter<C>,
        target: &mut T,
        command: XUpcasePacket<'a>,
    ) -> Result<HandlerStatus, Error<T::Error, C::Error>> {
        if !target.use_x_upcase_packet() {
            return Ok(HandlerStatus::Handled);
        }

        crate::__dead_code_marker!("x_upcase_packet", "impl");

        let handler_status = match command {
            XUpcasePacket::X(cmd) => {
                let addr = <T::Arch as Arch>::Usize::from_be_bytes(cmd.addr)
                    .ok_or(Error::TargetMismatch)?;

                match target.base_ops() {
                    BaseOps::SingleThread(ops) => ops.write_addrs(addr, cmd.val),
                    BaseOps::MultiThread(ops) => {
                        ops.write_addrs(addr, cmd.val, self.current_mem_tid)
                    }
                }
                .handle_error()?;

                HandlerStatus::NeedsOk
            }
        };
        Ok(handler_status)
    }
}
