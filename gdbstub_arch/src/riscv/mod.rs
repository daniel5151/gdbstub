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
    type BreakpointKind = usize;
    type RegId = reg::id::RiscvRegId<u32>;

    fn target_description_xml() -> Option<&'static str> {
        // Source: https://github.com/bminor/binutils-gdb/blob/master/gdb/features/riscv/32bit-cpu.xml
        // <!-- Copyright (C) 2018-2024 Free Software Foundation, Inc.
        //
        // Copying and distribution of this file, with or without modification,
        // are permitted in any medium without royalty provided the copyright
        // notice and this notice are preserved.  -->
        Some(r#"<target version="1.0">
<architecture>riscv:rv32</architecture>
<feature name="org.gnu.gdb.riscv.cpu">
  <reg name="zero" bitsize="32" type="int" regnum="0"/>
  <reg name="ra" bitsize="32" type="code_ptr"/>
  <reg name="sp" bitsize="32" type="data_ptr"/>
  <reg name="gp" bitsize="32" type="data_ptr"/>
  <reg name="tp" bitsize="32" type="data_ptr"/>
  <reg name="t0" bitsize="32" type="int"/>
  <reg name="t1" bitsize="32" type="int"/>
  <reg name="t2" bitsize="32" type="int"/>
  <reg name="fp" bitsize="32" type="data_ptr"/>
  <reg name="s1" bitsize="32" type="int"/>
  <reg name="a0" bitsize="32" type="int"/>
  <reg name="a1" bitsize="32" type="int"/>
  <reg name="a2" bitsize="32" type="int"/>
  <reg name="a3" bitsize="32" type="int"/>
  <reg name="a4" bitsize="32" type="int"/>
  <reg name="a5" bitsize="32" type="int"/>
  <reg name="a6" bitsize="32" type="int"/>
  <reg name="a7" bitsize="32" type="int"/>
  <reg name="s2" bitsize="32" type="int"/>
  <reg name="s3" bitsize="32" type="int"/>
  <reg name="s4" bitsize="32" type="int"/>
  <reg name="s5" bitsize="32" type="int"/>
  <reg name="s6" bitsize="32" type="int"/>
  <reg name="s7" bitsize="32" type="int"/>
  <reg name="s8" bitsize="32" type="int"/>
  <reg name="s9" bitsize="32" type="int"/>
  <reg name="s10" bitsize="32" type="int"/>
  <reg name="s11" bitsize="32" type="int"/>
  <reg name="t3" bitsize="32" type="int"/>
  <reg name="t4" bitsize="32" type="int"/>
  <reg name="t5" bitsize="32" type="int"/>
  <reg name="t6" bitsize="32" type="int"/>
  <reg name="pc" bitsize="32" type="code_ptr"/>
</feature>
</target>"#)
    }
}

impl Arch for Riscv64 {
    type Usize = u64;
    type Registers = reg::RiscvCoreRegs<u64>;
    type BreakpointKind = usize;
    type RegId = reg::id::RiscvRegId<u64>;

    fn target_description_xml() -> Option<&'static str> {
        // Source: https://github.com/bminor/binutils-gdb/blob/master/gdb/features/riscv/64bit-cpu.xml
        // <!-- Copyright (C) 2018-2024 Free Software Foundation, Inc.
        //
        // Copying and distribution of this file, with or without modification,
        // are permitted in any medium without royalty provided the copyright
        // notice and this notice are preserved.  -->

        Some(r#"<target version="1.0">
<architecture>riscv:rv64</architecture>
<feature name="org.gnu.gdb.riscv.cpu">
  <reg name="zero" bitsize="64" type="int" regnum="0"/>
  <reg name="ra" bitsize="64" type="code_ptr"/>
  <reg name="sp" bitsize="64" type="data_ptr"/>
  <reg name="gp" bitsize="64" type="data_ptr"/>
  <reg name="tp" bitsize="64" type="data_ptr"/>
  <reg name="t0" bitsize="64" type="int"/>
  <reg name="t1" bitsize="64" type="int"/>
  <reg name="t2" bitsize="64" type="int"/>
  <reg name="fp" bitsize="64" type="data_ptr"/>
  <reg name="s1" bitsize="64" type="int"/>
  <reg name="a0" bitsize="64" type="int"/>
  <reg name="a1" bitsize="64" type="int"/>
  <reg name="a2" bitsize="64" type="int"/>
  <reg name="a3" bitsize="64" type="int"/>
  <reg name="a4" bitsize="64" type="int"/>
  <reg name="a5" bitsize="64" type="int"/>
  <reg name="a6" bitsize="64" type="int"/>
  <reg name="a7" bitsize="64" type="int"/>
  <reg name="s2" bitsize="64" type="int"/>
  <reg name="s3" bitsize="64" type="int"/>
  <reg name="s4" bitsize="64" type="int"/>
  <reg name="s5" bitsize="64" type="int"/>
  <reg name="s6" bitsize="64" type="int"/>
  <reg name="s7" bitsize="64" type="int"/>
  <reg name="s8" bitsize="64" type="int"/>
  <reg name="s9" bitsize="64" type="int"/>
  <reg name="s10" bitsize="64" type="int"/>
  <reg name="s11" bitsize="64" type="int"/>
  <reg name="t3" bitsize="64" type="int"/>
  <reg name="t4" bitsize="64" type="int"/>
  <reg name="t5" bitsize="64" type="int"/>
  <reg name="t6" bitsize="64" type="int"/>
  <reg name="pc" bitsize="64" type="code_ptr"/>
</feature>
</target>"#)
    }
}
