use crate::emu::Emu;
use gdbstub::target;
use gdbstub::target::TargetResult;
use log::info;

impl target::ext::flash::Flash for Emu {
    fn flash_erase(&mut self, start_addr: u32, length: u32) -> TargetResult<(), Self> {
        info!("flash_erase start_addr: {start_addr:08x}, length: {length:08x}");
        Ok(())
    }

    fn flash_write(&mut self, start_addr: u32, _data: &[u8]) -> TargetResult<(), Self> {
        info!("flash_write start_addr: {start_addr:08x}");
        Ok(())
    }

    fn flash_done(&mut self) -> TargetResult<(), Self> {
        info!("flash_done");
        Ok(())
    }
}
