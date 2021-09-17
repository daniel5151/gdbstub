use gdbstub::target;
use gdbstub::target::TargetResult;

use crate::emu::Emu;

impl target::ext::memory_map::MemoryMap for Emu {
    fn memory_map_xml(
        &self,
        offset: u64,
        length: usize,
        buf: &mut [u8],
    ) -> TargetResult<usize, Self> {
        // Sample memory map, with RAM coverying the whole
        // memory space.
        let memory_map = r#"<?xml version="1.0"?>
<!DOCTYPE memory-map
    PUBLIC "+//IDN gnu.org//DTD GDB Memory Map V1.0//EN"
            "http://sourceware.org/gdb/gdb-memory-map.dtd">
<memory-map>
    <memory type="ram" start="0x0" length="0x100000000"/>
</memory-map>"#
            .trim()
            .as_bytes();

        let len = memory_map.len();
        let data = &memory_map[len.min(offset as usize)..len.min(offset as usize + length)];
        let buf = &mut buf[..data.len()];
        buf.copy_from_slice(data);
        Ok(data.len())
    }
}
