//! Implementations for various ARM architectures.

use gdbstub::arch::Arch;

pub mod reg;

/// ARM-specific breakpoint kinds.
///
/// Extracted from the GDB documentation at
/// [E.5.1.1 ARM Breakpoint Kinds](https://sourceware.org/gdb/current/onlinedocs/gdb/ARM-Breakpoint-Kinds.html#ARM-Breakpoint-Kinds)
#[derive(Debug)]
pub enum ArmBreakpointKind {
    /// 16-bit Thumb mode breakpoint.
    Thumb16,
    /// 32-bit Thumb mode (Thumb-2) breakpoint.
    Thumb32,
    /// 32-bit ARM mode breakpoint.
    Arm32,
}

impl gdbstub::arch::BreakpointKind for ArmBreakpointKind {
    fn from_usize(kind: usize) -> Option<Self> {
        let kind = match kind {
            2 => ArmBreakpointKind::Thumb16,
            3 => ArmBreakpointKind::Thumb32,
            4 => ArmBreakpointKind::Arm32,
            _ => return None,
        };
        Some(kind)
    }
}

/// Implements `Arch` for ARMv4T
pub enum Armv4t {}

impl Arch for Armv4t {
    type Usize = u32;
    type Registers = reg::ArmCoreRegs;
    type RegId = reg::id::ArmCoreRegId;
    type BreakpointKind = ArmBreakpointKind;

    fn target_description_xml() -> Option<&'static str> {
        Some(r#"<target version="1.0"><architecture>armv4t</architecture></target>"#)
    }
}
