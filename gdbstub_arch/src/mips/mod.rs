//! Implementations for the MIPS architecture.

use gdbstub::arch::Arch;
use gdbstub::arch::RegId;

pub mod reg;

/// MIPS-specific breakpoint kinds.
///
/// Extracted from the GDB documentation at
/// [E.5.1.1 MIPS Breakpoint Kinds](https://sourceware.org/gdb/current/onlinedocs/gdb/MIPS-Breakpoint-Kinds.html#MIPS-Breakpoint-Kinds)
#[derive(Debug)]
pub enum MipsBreakpointKind {
    /// 16-bit MIPS16 mode breakpoint.
    Mips16,

    /// 16-bit microMIPS mode breakpoint.
    MicroMips16,

    /// 32-bit standard MIPS mode breakpoint.
    Mips32,

    /// 32-bit microMIPS mode breakpoint.
    MicroMips32,
}

impl gdbstub::arch::BreakpointKind for MipsBreakpointKind {
    fn from_usize(kind: usize) -> Option<Self> {
        let kind = match kind {
            2 => MipsBreakpointKind::Mips16,
            3 => MipsBreakpointKind::MicroMips16,
            4 => MipsBreakpointKind::Mips32,
            5 => MipsBreakpointKind::MicroMips32,
            _ => return None,
        };
        Some(kind)
    }
}

/// Implements `Arch` for 32-bit MIPS.
///
/// Check out the [module level docs](gdbstub::arch#whats-with-regidimpl) for
/// more info about the `RegIdImpl` type parameter.
pub enum Mips<RegIdImpl: RegId = reg::id::MipsRegId<u32>> {
    #[doc(hidden)]
    _Marker(core::marker::PhantomData<RegIdImpl>),
}

/// Implements `Arch` for 64-bit MIPS.
///
/// Check out the [module level docs](gdbstub::arch#whats-with-regidimpl) for
/// more info about the `RegIdImpl` type parameter.
pub enum Mips64<RegIdImpl: RegId = reg::id::MipsRegId<u64>> {
    #[doc(hidden)]
    _Marker(core::marker::PhantomData<RegIdImpl>),
}

/// Implements `Arch` for 32-bit MIPS with the DSP feature enabled.
pub enum MipsWithDsp {}

/// Implements `Arch` for 64-bit MIPS with the DSP feature enabled.
pub enum Mips64WithDsp {}

impl<RegIdImpl: RegId> Arch for Mips<RegIdImpl> {
    type Usize = u32;
    type Registers = reg::MipsCoreRegs<u32>;
    type RegId = RegIdImpl;
    type BreakpointKind = MipsBreakpointKind;

    fn target_description_xml() -> Option<&'static str> {
        Some(r#"<target version="1.0"><architecture>mips</architecture></target>"#)
    }
}

impl<RegIdImpl: RegId> Arch for Mips64<RegIdImpl> {
    type Usize = u64;
    type Registers = reg::MipsCoreRegs<u64>;
    type RegId = RegIdImpl;
    type BreakpointKind = MipsBreakpointKind;

    fn target_description_xml() -> Option<&'static str> {
        Some(r#"<target version="1.0"><architecture>mips64</architecture></target>"#)
    }
}

impl Arch for MipsWithDsp {
    type Usize = u32;
    type Registers = reg::MipsCoreRegsWithDsp<u32>;
    type RegId = reg::id::MipsRegId<u32>;
    type BreakpointKind = MipsBreakpointKind;

    fn target_description_xml() -> Option<&'static str> {
        Some(
            r#"<target version="1.0"><architecture>mips</architecture><feature name="org.gnu.gdb.mips.dsp"></feature></target>"#,
        )
    }
}

impl Arch for Mips64WithDsp {
    type Usize = u64;
    type Registers = reg::MipsCoreRegsWithDsp<u64>;
    type RegId = reg::id::MipsRegId<u64>;
    type BreakpointKind = MipsBreakpointKind;

    fn target_description_xml() -> Option<&'static str> {
        Some(
            r#"<target version="1.0"><architecture>mips64</architecture><feature name="org.gnu.gdb.mips.dsp"></feature></target>"#,
        )
    }
}
