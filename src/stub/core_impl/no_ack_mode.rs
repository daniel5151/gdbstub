use super::prelude::*;
use crate::protocol::commands::ext::NoAckMode;

impl<T: Target, C: Connection> GdbStubImpl<T, C> {
    pub(crate) fn handle_no_ack_mode(
        &mut self,
        _res: &mut ResponseWriter<'_, C>,
        target: &mut T,
        command: NoAckMode,
    ) -> Result<HandlerStatus, Error<T::Error, C::Error>> {
        if !target.use_no_ack_mode() {
            return Ok(HandlerStatus::Handled);
        }

        crate::__dead_code_marker!("no_ack_mode", "impl");

        let handler_status = match command {
            NoAckMode::QStartNoAckMode(_) => {
                self.features.set_no_ack_mode(true);
                HandlerStatus::NeedsOk
            }
        };
        Ok(handler_status)
    }
}
