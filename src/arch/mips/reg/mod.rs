//! `GdbRegister` structs for MIPS architectures.

mod mips;

pub use mips::MipsCoreRegs;
pub use mips::MipsCp0Regs;
pub use mips::MipsFpuRegs;
