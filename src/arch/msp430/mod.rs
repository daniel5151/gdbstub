//! Implementations for the TI-MSP430 family of MCUs.

use crate::arch::Arch;

pub mod reg;

/// Implements `Arch` for standard 16-bit TI-MSP430 MCUs.
pub enum Msp430 {}

impl Arch for Msp430 {
    type Usize = u32;
    type Registers = reg::Msp430Regs;
    type RegId = reg::id::Msp430RegId;

    fn target_description_xml() -> Option<&'static str> {
        Some(r#"<target version="1.0"><architecture>msp430</architecture></target>"#)
    }
}
