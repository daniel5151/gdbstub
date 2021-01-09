//! Implementations for various PowerPC architectures.

use crate::arch::Arch;
use crate::arch::RegId;

pub mod reg;

/// Implements `Arch` for 32-bit PowerPC + AltiVec SIMD.
///
/// Check out the [module level docs](../index.html#whats-with-regidimpl) for
/// more info about the `RegIdImpl` type parameter.
pub enum PowerPcAltivec32<RegIdImpl: RegId> {
    #[doc(hidden)]
    _Marker(core::marker::PhantomData<RegIdImpl>),
}

impl<RegIdImpl: RegId> Arch for PowerPcAltivec32<RegIdImpl> {
    type Usize = u32;
    type Registers = reg::PowerPcCommonRegs;
    type RegId = RegIdImpl;

    fn target_description_xml() -> Option<&'static str> {
        Some(
            r#"<target version="1.0"><architecture>powerpc:common</architecture><feature name="org.gnu.gdb.power.core"></feature><feature name="org.gnu.gdb.power.fpu"></feature><feature name="org.gnu.gdb.power.altivec"></feature></target>"#,
        )
    }
}
