//! `Register` structs for various TI-MSP430 CPUs.

/// `RegId` definitions for various TI-MSP430 CPUs.
pub mod id;

mod msp430;

pub use msp430::Msp430Regs;
