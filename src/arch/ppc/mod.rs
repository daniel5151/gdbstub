//! Implementations for various PowerPC architectures.

use crate::arch::Arch;

pub mod reg;

/// Implements `Arch` for 32-bit PowerPC + AltiVec SIMD
#[derive(Eq, PartialEq)]
pub struct PowerPcAltivec32;

impl Arch for PowerPcAltivec32 {
    type Usize = u32;
    type Registers = reg::PowerPcCommonRegs;

    fn target_description_xml() -> Option<&'static str> {
        Some(
            r#"<target version="1.0"><architecture>powerpc:common</architecture><feature name="org.gnu.gdb.power.core"></feature><feature name="org.gnu.gdb.power.fpu"></feature><feature name="org.gnu.gdb.power.altivec"></feature></target>"#,
        )
    }
}
