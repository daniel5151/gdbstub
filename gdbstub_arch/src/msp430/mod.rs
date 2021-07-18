//! Implementations for the TI-MSP430 family of MCUs.

use gdbstub::arch::Arch;

pub mod reg;

/// Implements `Arch` for standard 16-bit TI-MSP430 MCUs.
pub struct Msp430 {}

impl Arch for Msp430 {
    type Usize = u16;
    type Registers = reg::Msp430Regs<u16>;
    type RegId = reg::id::Msp430RegId<u16>;
    type BreakpointKind = usize;

    fn target_description_xml() -> Option<&'static str> {
        Some(r#"<target version="1.0"><architecture>msp430</architecture></target>"#)
    }
}

/// Implements `Arch` for 20-bit TI-MSP430 MCUs (CPUX).
pub struct Msp430X {}

impl Arch for Msp430X {
    type Usize = u32;
    type Registers = reg::Msp430Regs<u32>;
    type RegId = reg::id::Msp430RegId<u32>;
    type BreakpointKind = usize;

    fn target_description_xml() -> Option<&'static str> {
        Some(r#"<target version="1.0"><architecture>msp430x</architecture></target>"#)
    }
}
