//! Get section/segment relocation offsets from the target.
//!
//! For some targets, sections may be relocated from their base address. As
//! a result, the stub may need to tell GDB the final section addresses
//! to ensure that debug symbols are resolved correctly after relocation.
//!
//! _Note:_ This extension corresponds to the `qOffsets` command, which is
//! limited to reporting the offsets for code, data and bss, and is
//! generally considered a legacy feature.
//!
//! For targets where library offsets are maintained externally (e.g. Windows)
//! you should consider implementing the more flexible `qXfer:library:read`.
//! See issue [#20](https://github.com/daniel5151/gdbstub/issues/20) for more
//! info.
//!
//! For System-V architectures GDB is capable of extracting library offsets
//! from memory if it knows the base address of the dynamic linker. The base
//! address can be specified by either implementing this command or by including
//! a `AT_BASE` entry in the response to the more modern `qXfer:auxv:read`
//! command. See issue [#20](https://github.com/daniel5151/gdbstub/issues/20)
//! for more info.

use crate::arch::Arch;
use crate::target::Target;

/// Describes the offset the target loaded the image sections at, so the target
/// can notify GDB that it needs to adjust the addresses of symbols.
///
/// GDB supports either section offsets, or segment addresses.
pub enum Offsets<U> {
    /// Section offsets relative to their base addresses.
    Sections {
        /// The offset of the `.text` section.
        text: U,
        /// The offset of the `.data` section.
        data: U,
        /// The offset of the `.bss` section.
        ///
        /// _Note:_ GDB expects that `bss` is either `None` or equal to `data`.
        bss: Option<U>,
    },

    /// Absolute addresses of the first two segments.
    ///
    /// _Note:_ any extra segments will kept at fixed offsets relative to the
    /// last relocated segment.
    Segments {
        /// The absolute address of the first segment which conventionally
        /// contains program code.
        text_seg: U,
        /// The absolute address of the second segment which conventionally
        /// contains modifiable data.
        data_seg: Option<U>,
    },
}

/// Target Extension - Get section/segment relocation offsets from the target.
///
/// Corresponds to the `qOffset` command. See the [section_offset module
/// documentation](index.html).
pub trait SectionOffsets: Target {
    /// Return the target's current section (or segment) offsets.
    fn get_section_offsets(&mut self) -> Result<Offsets<<Self::Arch as Arch>::Usize>, Self::Error>;
}

define_ext!(SectionOffsetsOps, SectionOffsets);
