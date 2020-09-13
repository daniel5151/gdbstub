//! Implementations for various x86 architectures.

use crate::arch::Arch;

pub mod reg;

/// Implements `Arch` for 64-bit x86 + SSE Extensions
#[allow(non_camel_case_types)]
pub enum X86_64_SSE {}

impl Arch for X86_64_SSE {
    type Usize = u64;
    type Registers = reg::X86_64CoreRegs;

    fn target_description_xml() -> Option<&'static str> {
        Some(
            r#"<target version="1.0"><architecture>i386:x86-64</architecture><feature name="org.gnu.gdb.i386.sse"></feature></target>"#,
        )
    }
}

/// Implements `Arch` for 32-bit x86 + SSE Extensions
#[allow(non_camel_case_types)]
pub enum X86_SSE {}

impl Arch for X86_SSE {
    type Usize = u32;
    type Registers = reg::X86CoreRegs;

    fn target_description_xml() -> Option<&'static str> {
        Some(
            r#"<target version="1.0"><architecture>i386:intel</architecture><feature name="org.gnu.gdb.i386.sse"></feature></target>"#,
        )
    }
}
