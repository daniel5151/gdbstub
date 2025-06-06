use super::copy_range_to_buf;
use crate::emu::Emu;
use gdbstub::target;
use gdbstub::target::TargetResult;

impl target::ext::memory_map::MemoryMap for Emu {
    fn memory_map_xml(
        &self,
        offset: u64,
        length: usize,
        buf: &mut [u8],
    ) -> TargetResult<usize, Self> {
        // Sample memory map, modeled on part of STM32F446 memory map.
        // A real memory map is necessary to test the flash commands.
        let memory_map = r#"<?xml version="1.0"?>
<!DOCTYPE memory-map
    PUBLIC "+//IDN gnu.org//DTD GDB Memory Map V1.0//EN"
            "http://sourceware.org/gdb/gdb-memory-map.dtd">
<memory-map>
    <memory type="ram" start="0x20000000" length="0x20000"/>
    <memory type="flash" start="0x08000000" length="0x10000">
        <property name="blocksize">0x4000</property>
    </memory>
    <memory type="flash" start="0x08010000" length="0x10000">
        <property name="blocksize">0x10000</property>
    </memory>
    <memory type="flash" start="0x08020000" length="0x60000">
        <property name="blocksize">0x20000</property>
    </memory>
</memory-map>"#
            .trim()
            .as_bytes();
        Ok(copy_range_to_buf(memory_map, offset, length, buf))
    }
}
