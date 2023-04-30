//! Provide a memory map for the target.
use crate::target::Target;
use crate::target::TargetResult;

/// Target Extension - Read the target's memory map.
pub trait MemoryMap: Target {
    /// Get memory map XML file from the target.
    ///
    /// See the [GDB Documentation] for a description of the format.
    ///
    /// [GDB Documentation]: https://sourceware.org/gdb/onlinedocs/gdb/Memory-Map-Format.html
    ///
    /// Return the number of bytes written into `buf` (which may be less than
    /// `length`).
    ///
    /// If `offset` is greater than the length of the underlying data, return
    /// `Ok(0)`.
    fn memory_map_xml(
        &self,
        offset: u64,
        length: usize,
        buf: &mut [u8],
    ) -> TargetResult<usize, Self>;
}

define_ext!(MemoryMapOps, MemoryMap);
