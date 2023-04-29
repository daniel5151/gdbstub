//! Implementation for the [AArch64](https://developer.arm.com/documentation/102374)
//! ARM architecture.
//!
//! See PR [#109](https://github.com/daniel5151/gdbstub/pull/109) for more info.
//!
//! *Note*: doesn't support the AArch32 execution mode.
//! *Note*: the target XML currently advertises all system registers to the GDB
//! client.

use gdbstub::arch::Arch;

pub mod reg;

/// Implements `Arch` for ARM AArch64.
pub struct AArch64 {}

impl Arch for AArch64 {
    type Usize = u64;
    type Registers = reg::AArch64CoreRegs;
    type RegId = reg::id::AArch64RegId;
    type BreakpointKind = usize;

    fn target_description_xml() -> Option<&'static str> {
        static DESCRIPTION_XML: &str = concat!(
            r#"<target version="1.0">"#,
            "<architecture>aarch64</architecture>",
            include_str!("core.xml"), // feature "org.gnu.gdb.aarch64.core"
            include_str!("fpu.xml"),  // feature "org.gnu.gdb.aarch64.fpu"
            include_str!("sysregs.xml"),
            "</target>",
        );

        Some(DESCRIPTION_XML)
    }
}
