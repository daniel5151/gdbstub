//! Provide a memory map for the target.
use crate::target::Target;

/// Target Extension - Provide a target memory map.
pub trait MemoryMap: Target {
    /// Return the target memory map XML file.
    ///
    /// See the [GDB Documentation] for a description of the format.
    ///
    /// [GDB Documentation]: https://sourceware.org/gdb/onlinedocs/gdb/Memory-Map-Format.html
    fn memory_map_xml(&self) -> &str;
}

define_ext!(MemoryMapOps, MemoryMap);
