use super::prelude::*;
use crate::protocol::commands::ext::Agent;

impl<T: Target, C: Connection> GdbStubImpl<T, C> {
    pub(crate) fn handle_agent(
        &mut self,
        _res: &mut ResponseWriter<C>,
        target: &mut T,
        command: Agent,
    ) -> Result<HandlerStatus, Error<T::Error, C::Error>> {
        let ops = match target.agent() {
            Some(ops) => ops,
            None => return Ok(HandlerStatus::Handled),
        };

        let handler_status = match command {
            Agent::QAgent(cmd) => {
                ops.enabled(cmd.value).map_err(Error::TargetError)?;
                HandlerStatus::NeedsOK
            }
        };

        Ok(handler_status)
    }
}
