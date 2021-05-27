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
                let xml = ops.memory_map_xml().trim();
                if cmd.offset >= xml.len() {
                    // no more data
                    res.write_str("l")?;
                } else if cmd.offset + cmd.len >= xml.len() {
                    // last little bit of data
                    res.write_str("l")?;
                    res.write_binary(&xml.as_bytes()[cmd.offset..])?
                } else {
                    // still more data
                    res.write_str("m")?;
                    res.write_binary(&xml.as_bytes()[cmd.offset..(cmd.offset + cmd.len)])?
                }

                HandlerStatus::Handled
            }
        };

        Ok(handler_status)
    }
}
