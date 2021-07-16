//! Implementations for the TI-MSP430 family of MCUs.

use gdbstub::arch::Arch;
use gdbstub::arch::RegId;

pub mod reg;

/// Implements `Arch` for standard 16-bit TI-MSP430 MCUs.
///
/// Check out the [module level docs](gdbstub::arch#whats-with-regidimpl) for
/// more info about the `RegIdImpl` type parameter.
pub enum Msp430<RegIdImpl: RegId = reg::id::Msp430RegId<u16>> {
    #[doc(hidden)]
    _Marker(core::marker::PhantomData<RegIdImpl>),
}

impl<RegIdImpl: RegId> Arch for Msp430<RegIdImpl> {
    type Usize = u16;
    type Registers = reg::Msp430Regs<u16>;
    type RegId = RegIdImpl;
    type BreakpointKind = usize;

    fn target_description_xml() -> Option<&'static str> {
        Some(r#"<target version="1.0"><architecture>msp430</architecture></target>"#)
    }
}

/// Implements `Arch` for 20-bit TI-MSP430 MCUs (CPUX).
///
/// Check out the [module level docs](gdbstub::arch#whats-with-regidimpl) for
/// more info about the `RegIdImpl` type parameter.
pub enum Msp430X<RegIdImpl: RegId = reg::id::Msp430RegId<u32>> {
    #[doc(hidden)]
    _Marker(core::marker::PhantomData<RegIdImpl>),
}

impl<RegIdImpl: RegId> Arch for Msp430X<RegIdImpl> {
    type Usize = u32;
    type Registers = reg::Msp430Regs<u32>;
    type RegId = RegIdImpl;
    type BreakpointKind = usize;

    fn target_description_xml() -> Option<&'static str> {
        Some(r#"<target version="1.0"><architecture>msp430x</architecture></target>"#)
    }
}
