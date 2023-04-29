//! Implementations for various x86 architectures.

use gdbstub::arch::Arch;

pub mod reg;

/// Implements `Arch` for 64-bit x86 + SSE Extensions.
#[allow(non_camel_case_types, clippy::upper_case_acronyms)]
pub enum X86_64_SSE {}

impl Arch for X86_64_SSE {
    type Usize = u64;
    type Registers = reg::X86_64CoreRegs;
    type RegId = reg::id::X86_64CoreRegId;
    type BreakpointKind = usize;

    fn target_description_xml() -> Option<&'static str> {
        Some(
            r#"<target version="1.0"><architecture>i386:x86-64</architecture><feature name="org.gnu.gdb.i386.sse"></feature></target>"#,
        )
    }
}

/// Implements `Arch` for 32-bit x86 + SSE Extensions.
#[allow(non_camel_case_types, clippy::upper_case_acronyms)]
pub enum X86_SSE {}

impl Arch for X86_SSE {
    type Usize = u32;
    type Registers = reg::X86CoreRegs;
    type RegId = reg::id::X86CoreRegId;
    type BreakpointKind = usize;

    fn target_description_xml() -> Option<&'static str> {
        Some(
            r#"<target version="1.0"><architecture>i386:intel</architecture><feature name="org.gnu.gdb.i386.sse"></feature></target>"#,
        )
    }
}
