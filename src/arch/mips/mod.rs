//! Implementations for the MIPS architecture.

use crate::arch::Arch;

pub mod reg;

/// Implements `Arch` for 32-bit MIPS.
#[derive(Eq, PartialEq)]
pub struct Mips;

/// Implements `Arch` for 64-bit MIPS.
#[derive(Eq, PartialEq)]
pub struct Mips64;

impl Arch for Mips {
    type Usize = u32;
    type Registers = reg::MipsCoreRegs;

    fn target_description_xml() -> Option<&'static str> {
        Some(r#"<target version="1.0"><architecture>mips</architecture></target>"#)
    }
}

impl Arch for Mips64 {
    type Usize = u64;
    type Registers = reg::Mips64CoreRegs;

    fn target_description_xml() -> Option<&'static str> {
        Some(r#"<target version="1.0"><architecture>mips64</architecture></target>"#)
    }
}
