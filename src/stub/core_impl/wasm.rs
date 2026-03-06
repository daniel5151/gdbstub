use super::prelude::*;
use crate::protocol::commands::ext::Wasm;
use crate::protocol::IdKind;

impl<T: Target, C: Connection> GdbStubImpl<T, C> {
    pub(crate) fn handle_wasm(
        &mut self,
        res: &mut ResponseWriter<'_, C>,
        target: &mut T,
        command: Wasm,
    ) -> Result<HandlerStatus, Error<T::Error, C::Error>> {
        let ops = match target.support_wasm() {
            Some(ops) => ops,
            None => return Ok(HandlerStatus::Handled),
        };

        crate::__dead_code_marker!("wasm", "impl");

        match command {
            Wasm::qWasmCallStack(cmd) => {
                let mut error: Result<(), Error<T::Error, C::Error>> = Ok(());
                let tid = match cmd.tid.tid {
                    IdKind::All => {
                        return Err(Error::NonFatalError(1));
                    }
                    IdKind::Any => self.current_mem_tid,
                    IdKind::WithId(id) => id,
                };
                ops.wasm_call_stack(tid, &mut |pc| {
                    if let Err(e) = res.write_hex_buf(&pc.to_le_bytes()) {
                        error = Err(e.into());
                    }
                })
                .map_err(Error::TargetError)?;
                error?;
            }
            Wasm::qWasmLocal(cmd) => {
                let mut buf = [0u8; 16];
                let len = ops
                    .read_wasm_local(self.current_mem_tid, cmd.frame, cmd.local, &mut buf)
                    .map_err(Error::TargetError)?;
                res.write_hex_buf(&buf[0..len])?;
            }
            Wasm::qWasmGlobal(cmd) => {
                let mut buf = [0u8; 16];
                let len = ops
                    .read_wasm_global(self.current_mem_tid, cmd.frame, cmd.global, &mut buf)
                    .map_err(Error::TargetError)?;
                res.write_hex_buf(&buf[0..len])?;
            }
            Wasm::qWasmStackValue(cmd) => {
                let mut buf = [0u8; 16];
                let len = ops
                    .read_wasm_stack(self.current_mem_tid, cmd.frame, cmd.index, &mut buf)
                    .map_err(Error::TargetError)?;
                res.write_hex_buf(&buf[0..len])?;
            }
        };

        Ok(HandlerStatus::Handled)
    }
}
