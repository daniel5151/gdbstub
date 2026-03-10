use super::prelude::*;
use crate::protocol::commands::ext::LldbErrorStrings;

impl<T: Target, C: Connection> GdbStubImpl<T, C> {
    pub(crate) fn handle_lldb_error_strings(
        &mut self,
        _res: &mut ResponseWriter<'_, C>,
        _target: &mut T,
        command: LldbErrorStrings,
    ) -> Result<HandlerStatus, Error<T::Error, C::Error>> {
        match command {
            LldbErrorStrings::QEnableErrorStrings(_) => {
                self.features.set_lldb_error_strings(true);
                Ok(HandlerStatus::NeedsOk)
            }
        }
    }
}
