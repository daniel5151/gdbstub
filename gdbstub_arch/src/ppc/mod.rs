//! Implementations for various PowerPC architectures.

use gdbstub::arch::{Arch, RegId, SingleStepGdbBehavior};

pub mod reg;

/// Implements `Arch` for 32-bit PowerPC + AltiVec SIMD.
///
/// Check out the [module level docs](gdbstub::arch#whats-with-regidimpl) for
/// more info about the `RegIdImpl` type parameter.
pub enum PowerPcAltivec32<RegIdImpl: RegId> {
    #[doc(hidden)]
    _Marker(core::marker::PhantomData<RegIdImpl>),
}

impl<RegIdImpl: RegId> Arch for PowerPcAltivec32<RegIdImpl> {
    type Usize = u32;
    type Registers = reg::PowerPcCommonRegs;
    type RegId = RegIdImpl;
    type BreakpointKind = usize;

    fn target_description_xml() -> Option<&'static str> {
        Some(
            r#"<target version="1.0"><architecture>powerpc:common</architecture><feature name="org.gnu.gdb.power.core"></feature><feature name="org.gnu.gdb.power.fpu"></feature><feature name="org.gnu.gdb.power.altivec"></feature></target>"#,
        )
    }

    #[inline(always)]
    fn single_step_gdb_behavior() -> SingleStepGdbBehavior {
        SingleStepGdbBehavior::Required
    }
}
