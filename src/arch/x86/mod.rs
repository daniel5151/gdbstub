//! Implementations for various x86 architectures.

use crate::arch::Arch;
use crate::arch::RegId;

pub mod reg;

/// Implements `Arch` for 64-bit x86 + SSE Extensions.
///
/// Check out the [module level docs](../index.html#whats-with-regidimpl) for
/// more info about the `RegIdImpl` type parameter.
#[allow(non_camel_case_types)]
pub enum X86_64_SSE<RegIdImpl: RegId = reg::id::X86_64CoreRegId> {
    #[doc(hidden)]
    _Marker(core::marker::PhantomData<RegIdImpl>),
}

impl<RegIdImpl: RegId> Arch for X86_64_SSE<RegIdImpl> {
    type Usize = u64;
    type Registers = reg::X86_64CoreRegs;
    type RegId = RegIdImpl;

    fn target_description_xml() -> Option<&'static str> {
        Some(
            r#"<target version="1.0"><architecture>i386:x86-64</architecture><feature name="org.gnu.gdb.i386.sse"></feature></target>"#,
        )
    }
}

/// Implements `Arch` for 32-bit x86 + SSE Extensions.
///
/// Check out the [module level docs](../index.html#whats-with-regidimpl) for
/// more info about the `RegIdImpl` type parameter.
#[allow(non_camel_case_types)]
pub enum X86_SSE<RegIdImpl: RegId = reg::id::X86CoreRegId> {
    #[doc(hidden)]
    _Marker(core::marker::PhantomData<RegIdImpl>),
}

impl<RegIdImpl: RegId> Arch for X86_SSE<RegIdImpl> {
    type Usize = u32;
    type Registers = reg::X86CoreRegs;
    type RegId = RegIdImpl;

    fn target_description_xml() -> Option<&'static str> {
        Some(
            r#"<target version="1.0"><architecture>i386:intel</architecture><feature name="org.gnu.gdb.i386.sse"></feature></target>"#,
        )
    }
}
