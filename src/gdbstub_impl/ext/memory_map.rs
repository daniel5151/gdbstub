use super::prelude::*;
use crate::protocol::commands::ext::MemoryMap;

impl<T: Target, C: Connection> GdbStubImpl<T, C> {
    pub(crate) fn handle_memory_map(
        &mut self,
        res: &mut ResponseWriter<C>,
        target: &mut T,
        command: MemoryMap,
    ) -> Result<HandlerStatus, Error<T::Error, C::Error>> {
        let ops = match target.memory_map() {
            Some(ops) => ops,
            None => return Ok(HandlerStatus::Handled),
        };

        crate::__dead_code_marker!("memory_map", "impl");

        let handler_status = match command {
            MemoryMap::qXferMemoryMapRead(cmd) => {
                let xml = ops.memory_map_xml().trim().as_bytes();
                res.write_binary_range(xml, cmd.offset, cmd.len)?;
                HandlerStatus::Handled
            }
        };

        Ok(handler_status)
    }
}
