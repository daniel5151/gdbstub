//! Provide a memory map for the target.
use crate::target::{Target, TargetResult};

/// Target Extension - Provide a target memory map.
pub trait MemoryMap: Target {
    /// Return the target memory map XML file.
    ///
    /// See the [GDB Documentation] for a description of the format.
    ///
    /// [GDB Documentation]: https://sourceware.org/gdb/onlinedocs/gdb/Memory-Map-Format.html
    fn memory_map_xml(
        &self,
        offset: u64,
        length: usize,
        buf: &mut [u8],
    ) -> TargetResult<usize, Self>;
}

define_ext!(MemoryMapOps, MemoryMap);
