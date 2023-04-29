//! Implementations for the MIPS architecture.

use gdbstub::arch::Arch;

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
pub enum Mips {}

/// Implements `Arch` for 32-bit MIPS, with the DSP feature enabled.
pub enum MipsWithDsp {}

/// Implements `Arch` for 64-bit MIPS.
///
/// **NOTE:** Due to GDB client behavior, this arch does _not_ include a
/// built-in `target.xml` implementation. Consider manually implementing
/// [`TargetDescriptionXmlOverride`].
///
/// See [daniel5151/gdbstub#97](https://github.com/daniel5151/gdbstub/issues/97).
///
/// [`TargetDescriptionXmlOverride`]: gdbstub::target::ext::target_description_xml_override::TargetDescriptionXmlOverride
pub enum Mips64 {}

/// Implements `Arch` for 64-bit MIPS, with the DSP feature enabled.
///
/// **NOTE:** Due to GDB client behavior, this arch does _not_ include a
/// built-in `target.xml` implementation. Consider manually implementing
/// [`TargetDescriptionXmlOverride`].
///
/// See [daniel5151/gdbstub#97](https://github.com/daniel5151/gdbstub/issues/97).
///
/// [`TargetDescriptionXmlOverride`]: gdbstub::target::ext::target_description_xml_override::TargetDescriptionXmlOverride
pub enum Mips64WithDsp {}

impl Arch for Mips {
    type Usize = u32;
    type Registers = reg::MipsCoreRegs<u32>;
    type RegId = reg::id::MipsRegId<u32>;
    type BreakpointKind = MipsBreakpointKind;

    fn target_description_xml() -> Option<&'static str> {
        Some(r#"<target version="1.0"><architecture>mips</architecture></target>"#)
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

#[allow(deprecated)]
impl Arch for Mips64 {
    type Usize = u64;
    type Registers = reg::MipsCoreRegs<u64>;
    type RegId = reg::id::MipsRegId<u64>;
    type BreakpointKind = MipsBreakpointKind;

    fn target_description_xml() -> Option<&'static str> {
        None
    }
}

#[allow(deprecated)]
impl Arch for Mips64WithDsp {
    type Usize = u64;
    type Registers = reg::MipsCoreRegsWithDsp<u64>;
    type RegId = reg::id::MipsRegId<u64>;
    type BreakpointKind = MipsBreakpointKind;

    fn target_description_xml() -> Option<&'static str> {
        None
    }
}
