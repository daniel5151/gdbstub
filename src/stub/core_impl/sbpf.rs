use super::prelude::*;
use crate::protocol::commands::ext::Sbpf;

impl<T: Target, C: Connection> GdbStubImpl<T, C> {
    pub(crate) fn handle_sbpf(
        &mut self,
        res: &mut ResponseWriter<'_, C>,
        target: &mut T,
        command: Sbpf,
    ) -> Result<HandlerStatus, Error<T::Error, C::Error>> {
        let ops = match target.support_sbpf() {
            Some(ops) => ops,
            None => return Ok(HandlerStatus::Handled),
        };

        crate::__dead_code_marker!("sbpf", "impl");

        match command {
            Sbpf::qSBPFCallStack(cmd) => {
                let mut error: Result<(), Error<T::Error, C::Error>> = Ok(());
                ops.sbpf_call_stack(cmd.tid.tid, &mut |pc, frame_pointer| {
                    if let Err(e) = res.write_hex_buf(&pc.to_le_bytes()) {
                        error = Err(e.into());
                    }
                    if let Err(e) = res.write_hex_buf(&frame_pointer.to_le_bytes()) {
                        error = Err(e.into());
                    }
                })
                .map_err(Error::TargetError)?;
                error?;
            }
        }

        Ok(HandlerStatus::Handled)
    }
}
