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
                // TODO: plumb through PID when true multi-process support is added
                let _pid = cmd.tid.pid;
                let Some(thread_id) = T::Tid::from_fully_qualified_tid(cmd.tid.tid) else {
                    return Err(Error::UnexpectedThreadId);
                };

                let mut error: Result<(), Error<T::Error, C::Error>> = Ok(());
                ops.wasm_call_stack(thread_id, &mut |pc| {
                    if let Err(e) = res.write_hex_buf(&pc.to_le_bytes()) {
                        error = Err(e.into());
                    }
                })
                .map_err(Error::TargetError)?;
                error?;
            }
            Wasm::qWasmLocal(cmd) => {
                // TODO: plumb through PID when true multi-process support is added
                let Some(thread_id) = T::Tid::from_fully_qualified_tid(self.current_mem_tid) else {
                    return Err(Error::UnexpectedThreadId);
                };

                let len = ops
                    .read_wasm_local(thread_id, cmd.frame, cmd.local, cmd.buf)
                    .map_err(Error::TargetError)?;
                res.write_hex_buf(&cmd.buf[0..len])?;
            }
            Wasm::qWasmGlobal(cmd) => {
                // TODO: plumb through PID when true multi-process support is added
                let Some(thread_id) = T::Tid::from_fully_qualified_tid(self.current_mem_tid) else {
                    return Err(Error::UnexpectedThreadId);
                };

                let len = ops
                    .read_wasm_global(thread_id, cmd.frame, cmd.global, cmd.buf)
                    .map_err(Error::TargetError)?;
                res.write_hex_buf(&cmd.buf[0..len])?;
            }
            Wasm::qWasmStackValue(cmd) => {
                // TODO: plumb through PID when true multi-process support is added
                let Some(thread_id) = T::Tid::from_fully_qualified_tid(self.current_mem_tid) else {
                    return Err(Error::UnexpectedThreadId);
                };

                let len = ops
                    .read_wasm_stack(thread_id, cmd.frame, cmd.index, cmd.buf)
                    .map_err(Error::TargetError)?;
                res.write_hex_buf(&cmd.buf[0..len])?;
            }
        };

        Ok(HandlerStatus::Handled)
    }
}
