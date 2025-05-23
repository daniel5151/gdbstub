//! Provide flash operations on the target.
use crate::arch::Arch;
use crate::target::Target;
use crate::target::TargetResult;

/// Flash memory operations.
pub trait Flash: Target {
    /// Erase `length` bytes of the target's flash memory starting from
    /// `start_addr`.
    ///
    /// GDB ensures `start_addr` and `length` are aligned to flash memory
    /// sectors as defined by the memory map xml.
    fn flash_erase(
        &mut self,
        start_addr: <Self::Arch as Arch>::Usize,
        length: <Self::Arch as Arch>::Usize,
    ) -> TargetResult<(), Self>;

    /// Write bytes to the target'a flash memory.
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
