use super::prelude::*;
use crate::protocol::commands::ext::Wasm;

impl<T: Target, C: Connection> GdbStubImpl<T, C> {
    pub(crate) fn handle_wasm(
        &mut self,
        res: &mut ResponseWriter<'_, C>,
        target: &mut T,
        command: Wasm<'_>,
    ) -> Result<HandlerStatus, Error<T::Error, C::Error>> {
        let ops = match target.support_wasm() {
            Some(ops) => ops,
            None => return Ok(HandlerStatus::Handled),
        };

        crate::__dead_code_marker!("wasm", "impl");

        match command {
            Wasm::qWasmCallStack(cmd) => {
                let mut error: Result<(), Error<T::Error, C::Error>> = Ok(());
                ops.wasm_call_stack(cmd.tid.tid, &mut |pc| {
                    if let Err(e) = res.write_hex_buf(&pc.to_le_bytes()) {
                        error = Err(e.into());
                    }
                })
                .map_err(Error::TargetError)?;
                error?;
            }
            Wasm::qWasmLocal(cmd) => {
                let len = ops
                    .read_wasm_local(self.current_mem_tid, cmd.frame, cmd.local, cmd.buf)
                    .map_err(Error::TargetError)?;
                res.write_hex_buf(&cmd.buf[0..len])?;
            }
            Wasm::qWasmGlobal(cmd) => {
                let len = ops
                    .read_wasm_global(self.current_mem_tid, cmd.frame, cmd.global, cmd.buf)
                    .map_err(Error::TargetError)?;
                res.write_hex_buf(&cmd.buf[0..len])?;
            }
            Wasm::qWasmStackValue(cmd) => {
                let len = ops
                    .read_wasm_stack(self.current_mem_tid, cmd.frame, cmd.index, cmd.buf)
                    .map_err(Error::TargetError)?;
                res.write_hex_buf(&cmd.buf[0..len])?;
            }
        };

        Ok(HandlerStatus::Handled)
    }
}
