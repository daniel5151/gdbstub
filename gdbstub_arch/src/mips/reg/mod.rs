//! `Register` structs for MIPS architectures.

/// `RegId` definitions for MIPS architectures.
pub mod id;

mod mips;

pub use mips::MipsCoreRegs;
pub use mips::MipsCoreRegsWithDsp;
pub use mips::MipsCp0Regs;
pub use mips::MipsFpuRegs;
