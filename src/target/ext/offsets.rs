//! Handle the `qOffsets` command

use crate::target::Target;

/// Target Extension - Handle the `qOffsets` command
pub trait OffsetsCmd: Target {
    /// Handle the `qOffsets` command
    ///
    /// For some targets, sections may be relocated from their base address. As
    /// a result, the stub may need to tell GDB the final section addresses
    /// to ensure that debug symbols are resolved correctly after relocation.
    ///
    /// Implementing this command allows the stub to report text, data, and bss
    /// offsets to GDB.
    fn get_section_offsets(
        &mut self,
    ) -> Result<crate::SectionOffsets<<Self::Arch as crate::target::Arch>::Usize>, Self::Error>;
}
