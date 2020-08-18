//! Implementations for x86

use crate::arch::Arch;

pub mod reg;

/// Implements `Arch` for 64-bit x86
#[derive(Eq, PartialEq)]
pub struct X86_64;

impl Arch for X86_64 {
    type Usize = u64;
    type Registers = reg::Amd64CoreRegs;

    fn target_description_xml() -> Option<&'static str> {
        Some(r#"<target version="1.0"><architecture>i386:x86-64</architecture></target>"#)
    }
}
