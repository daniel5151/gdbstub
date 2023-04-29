//! Implementations for the [RISC-V](https://riscv.org/) architecture.
//!
//! *Note*: currently only supports integer versions of the ISA.

use gdbstub::arch::Arch;

pub mod reg;

/// Implements `Arch` for 32-bit RISC-V.
pub enum Riscv32 {}

/// Implements `Arch` for 64-bit RISC-V.
pub enum Riscv64 {}

impl Arch for Riscv32 {
    type Usize = u32;
    type Registers = reg::RiscvCoreRegs<u32>;
    type RegId = reg::id::RiscvRegId<u32>;
    type BreakpointKind = usize;

    fn target_description_xml() -> Option<&'static str> {
        Some(r#"<target version="1.0"><architecture>riscv:rv32</architecture></target>"#)
    }
}

impl Arch for Riscv64 {
    type Usize = u64;
    type Registers = reg::RiscvCoreRegs<u64>;
    type RegId = reg::id::RiscvRegId<u64>;
    type BreakpointKind = usize;

    fn target_description_xml() -> Option<&'static str> {
        Some(r#"<target version="1.0"><architecture>riscv:rv64</architecture></target>"#)
    }
}
