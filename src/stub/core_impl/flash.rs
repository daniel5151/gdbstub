use super::prelude::*;
use crate::arch::Arch;
use crate::protocol::commands::ext::FlashOperations;

impl<T: Target, C: Connection> GdbStubImpl<T, C> {
    pub(crate) fn handle_flash_operations(
        &mut self,
        _res: &mut ResponseWriter<'_, C>,
        target: &mut T,
        command: FlashOperations<'_>,
    ) -> Result<HandlerStatus, Error<T::Error, C::Error>> {
        let ops = match target.support_flash_operations() {
            Some(ops) => ops,
            None => return Ok(HandlerStatus::Handled),
        };
        let handler_status = match command {
            FlashOperations::vFlashErase(cmd) => {
                let addr = <T::Arch as Arch>::Usize::from_be_bytes(cmd.addr)
                    .ok_or(Error::TargetMismatch)?;

                let length = <T::Arch as Arch>::Usize::from_be_bytes(cmd.length)
                    .ok_or(Error::TargetMismatch)?;

                ops.flash_erase(addr, length).handle_error()?;
                HandlerStatus::NeedsOk
            }
            FlashOperations::vFlashWrite(cmd) => {
                let addr = <T::Arch as Arch>::Usize::from_be_bytes(cmd.addr)
                    .ok_or(Error::TargetMismatch)?;

                ops.flash_write(addr, cmd.val).handle_error()?;
                HandlerStatus::NeedsOk
            }
            FlashOperations::vFlashDone(_) => {
                ops.flash_done().handle_error()?;
                HandlerStatus::NeedsOk
            }
        };

        Ok(handler_status)
    }
}
