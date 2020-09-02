//! Implementations for x86

use crate::arch::Arch;

pub mod reg;

/// Implements `Arch` for 64-bit x86
#[derive(Eq, PartialEq)]
pub struct X86_64;

impl Arch for X86_64 {
    type Usize = u64;
    type Registers = reg::X86_64CoreRegs;

    fn target_description_xml() -> Option<&'static str> {
        Some(
            r#"<target version="1.0"><architecture>i386:x86-64</architecture><feature name="org.gnu.gdb.i386.sse"></feature></target>"#,
        )
    }
}

/// Implements `Arch` for 32-bit x86
#[derive(Eq, PartialEq)]
pub struct X86;

impl Arch for X86 {
    type Usize = u32;
    type Registers = reg::X86CoreRegs;

    fn target_description_xml() -> Option<&'static str> {
        Some(
            r#"<target version="1.0"><architecture>i386:intel</architecture><feature name="org.gnu.gdb.i386.sse"></feature></target>"#,
        )
    }
}
