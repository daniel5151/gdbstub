//! Implementations for various ARM architectures.

use crate::arch::Arch;

pub mod reg;

/// Implements `Arch` for ARMv4T
pub enum Armv4t {}

impl Arch for Armv4t {
    type Usize = u32;
    type Registers = reg::ArmCoreRegs;
    type RegId = reg::id::ArmCoreRegId;

    fn target_description_xml() -> Option<&'static str> {
        Some(r#"<target version="1.0"><architecture>armv4t</architecture></target>"#)
    }
}
