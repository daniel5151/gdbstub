//! Provide flash operations on the target.
use crate::arch::Arch;
use crate::target::Target;
use crate::target::TargetResult;

/// Flash memory operations.
/// It's necessary to implement this extension to support GDB `load` command.
///
/// Typically, a GDB `load` command sequence starts by issuing a `flash_erase`
/// command, followed by multiple `flash_write` commands (typically one for each
/// loadable ELF section), and ends with a `flash_done` command.
///
/// The regions containing the addresses to be flashed must be specified as
/// "flash" regions in the memory map xml, returned by
/// [MemoryMap::memory_map_xml][crate::target::ext::memory_map::MemoryMap::memory_map_xml].
pub trait Flash: Target {
    /// Erase `length` bytes of the target's flash memory starting from
    /// `start_addr`.
    ///
    /// GDB ensures `start_addr` and `length` are aligned to flash memory
    /// block boundaries as defined by the memory map xml.
    fn flash_erase(
        &mut self,
        start_addr: <Self::Arch as Arch>::Usize,
        length: <Self::Arch as Arch>::Usize,
    ) -> TargetResult<(), Self>;

    /// Write bytes to the target's flash memory.
    ///
    /// GDB guarantees that the memory ranges specified by `flash_write`
    /// commands sent before a `flash_done` do not overlap and appear in
    /// order of increasing addresses.
    ///
    /// See [GDB Documentation] for more details.
    ///
    /// [GDB Documentation]: https://sourceware.org/gdb/current/onlinedocs/gdb.html/Packets.html
    fn flash_write(
        &mut self,
        start_addr: <Self::Arch as Arch>::Usize,
        data: &[u8],
    ) -> TargetResult<(), Self>;

    /// Indicate to the target that flash programming is finished.
    ///
    /// By GDB documentation, you can batch flash erase and write operations
    /// until this is called.
    fn flash_done(&mut self) -> TargetResult<(), Self>;
}

define_ext!(FlashOps, Flash);
