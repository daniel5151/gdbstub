//! `GdbRegister` structs for MIPS architectures.

mod mips;
mod mips64;

pub use mips::MipsCoreRegs;
pub use mips::MipsCp0Regs;
pub use mips::MipsFpuRegs;
pub use mips64::Mips64CoreRegs;
pub use mips64::Mips64Cp0Regs;
pub use mips64::Mips64FpuRegs;
