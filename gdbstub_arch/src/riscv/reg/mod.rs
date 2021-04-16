//! `Register` structs for RISC-V architectures.

/// `RegId` definitions for RISC-V architectures.
pub mod id;

mod riscv;

pub use riscv::RiscvCoreRegs;
