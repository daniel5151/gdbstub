//! Support for the [RISC-V](https://riscv.org/) architecture.
//!
//! *Note*: currently only supports integer version of the ISA

use crate::arch::Arch;

pub mod reg;

/// Implements `Arch` for 32-bit RISC-V.
#[derive(Eq, PartialEq)]
pub struct Riscv32;

/// Implements `Arch` for 64-bit RISC-V.
#[derive(Eq, PartialEq)]
pub struct Riscv64;

impl Arch for Riscv32 {
    type Usize = u32;
    type Registers = reg::RiscvCoreRegs<u32>;

    fn target_description_xml() -> Option<&'static str> {
        Some(r#"<target version="1.0"><architecture>riscv</architecture></target>"#)
    }
}

impl Arch for Riscv64 {
    type Usize = u64;
    type Registers = reg::RiscvCoreRegs<u64>;

    fn target_description_xml() -> Option<&'static str> {
        Some(r#"<target version="1.0"><architecture>riscv64</architecture></target>"#)
    }
}
