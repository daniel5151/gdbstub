use crate::gdb::Emu;
use gdbstub::target;
use gdbstub::target::ext::section_offsets::Offsets;

// This implementation is for illustrative purposes only. If the offsets are
// guaranteed to be zero, this extension does not need to be implemented.

impl target::ext::section_offsets::SectionOffsets for Emu {
    fn get_section_offsets(&mut self) -> Result<Offsets<u32>, Self::Error> {
        Ok(Offsets::Sections {
            text: 0,
            data: 0,
            bss: None,
        })
    }
}
