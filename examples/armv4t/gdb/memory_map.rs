use gdbstub::target;
use gdbstub::target::TargetResult;

use super::copy_range_to_buf;
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
        Ok(copy_range_to_buf(memory_map, offset, length, buf))
    }
}
