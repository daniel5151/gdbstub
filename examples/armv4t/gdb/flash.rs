use crate::emu::Emu;
use gdbstub::arch::Arch;
use gdbstub::target;
use gdbstub::target::TargetResult;

impl target::ext::flash::Flash for Emu {
    fn flash_erase(
        &mut self,
        start_addr: <Self::Arch as Arch>::Usize,
        length: <Self::Arch as Arch>::Usize,
    ) -> TargetResult<(), Self> {
        Ok(())
    }

    fn flash_write(
        &mut self,
        start_addr: <Self::Arch as Arch>::Usize,
        data: &[u8],
    ) -> TargetResult<(), Self> {
        Ok(())
    }

    fn flash_done(&mut self) -> TargetResult<(), Self> {
        Ok(())
    }
}
