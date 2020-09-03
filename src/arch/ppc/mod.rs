//! Implementations for various PowerPC architectures.

use crate::arch::Arch;

pub mod reg;

/// Implements `Arch` for 64-bit PowerPC
#[derive(Eq, PartialEq)]
pub struct PowerPc;

impl Arch for PowerPc {
    type Usize = u32;
    type Registers = reg::PowerPcCoreRegs;

    fn target_description_xml() -> Option<&'static str> {
        Some(r#"<target version="1.0"><architecture>powerpc:common</architecture></target>"#)
    }
}
