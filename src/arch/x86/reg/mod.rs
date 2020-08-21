//! `GdbRegister` structs for x86 architectures.

mod core64;

pub use core64::X86_64CoreRegs;
pub use core64::X87FpuInternalRegs;

/// 80-bit floating point value
pub type F80 = [u8; 10];
