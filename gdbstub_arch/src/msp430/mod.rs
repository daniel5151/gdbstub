//! Implementations for the TI-MSP430 family of MCUs.

use gdbstub::arch::Arch;
use gdbstub::arch::RegId;

pub mod reg;

/// Implements `Arch` for standard 16-bit TI-MSP430 MCUs.
///
/// Check out the [module level docs](gdbstub::arch#whats-with-regidimpl) for
/// more info about the `RegIdImpl` type parameter.
pub enum Msp430<RegIdImpl: RegId = reg::id::Msp430RegId> {
    #[doc(hidden)]
    _Marker(core::marker::PhantomData<RegIdImpl>),
}

impl<RegIdImpl: RegId> Arch for Msp430<RegIdImpl> {
    type Usize = u16;
    type Registers = reg::Msp430Regs;
    type RegId = RegIdImpl;
    type BreakpointKind = usize;

    fn target_description_xml() -> Option<&'static str> {
        Some(r#"<target version="1.0"><architecture>msp430</architecture></target>"#)
    }
}
