use super::prelude::*;
use crate::protocol::commands::ext::HostInfo;

use super::info_response::InfoResponse;
use crate::target::ext::host_info::InfoResponse as HostInfoResponse;

impl<T: Target, C: Connection> GdbStubImpl<T, C> {
    pub(crate) fn handle_host_info(
        &mut self,
        res: &mut ResponseWriter<'_, C>,
        target: &mut T,
        command: HostInfo,
    ) -> Result<HandlerStatus, Error<T::Error, C::Error>> {
        let ops = match target.support_host_info() {
            Some(ops) => ops,
            None => return Ok(HandlerStatus::Handled),
        };

        crate::__dead_code_marker!("host_info", "impl");

        let mut result = Ok(());
        let mut write_info = |info: &HostInfoResponse<'_>| {
            if result.is_ok() {
                if let Err(e) = InfoResponse::from(info).write_response(res) {
                    result = Err(e);
                }
            }
        };

        match command {
            HostInfo::qHostInfo(_cmd) => {
                ops.host_info(&mut write_info).map_err(Error::TargetError)?;
                result?;
            }
        };

        Ok(HandlerStatus::Handled)
    }
}
