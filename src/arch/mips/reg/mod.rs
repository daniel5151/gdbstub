//! `Register` structs for MIPS architectures.

/// `RegId` definitions for MIPS architectures.
pub mod id;

mod mips;

pub use mips::MipsCoreRegs;
