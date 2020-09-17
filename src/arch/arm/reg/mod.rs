//! `Register` structs for various ARM architectures.

/// `RegId` definitions for ARM architectures.
pub mod id;

mod arm_core;

pub use arm_core::ArmCoreRegs;
