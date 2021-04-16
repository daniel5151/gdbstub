//! `Register` structs for PowerPC architectures

/// `RegId` definitions for PowerPC architectures.
pub mod id;

mod common;

pub use common::PowerPcCommonRegs;
type PpcVector = u128;
