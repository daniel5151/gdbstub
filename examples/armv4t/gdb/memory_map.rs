use gdbstub::target;

use crate::emu::Emu;

impl target::ext::memory_map::MemoryMap for Emu {
    fn memory_map_xml(&self) -> &str {
        // Sample memory map, with RAM coverying the whole
        // memory space.
        r#"<?xml version="1.0"?>
<!DOCTYPE memory-map
    PUBLIC "+//IDN gnu.org//DTD GDB Memory Map V1.0//EN"
            "http://sourceware.org/gdb/gdb-memory-map.dtd">
<memory-map>
    <memory type="ram" start="0x0" length="0x100000000"/>
</memory-map>"#
    }
}
