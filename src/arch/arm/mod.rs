//! Implementations for various ARM architectures.

use crate::Arch;

pub mod reg;

/// Implements `Arch` for ARMv4T
#[derive(Eq, PartialEq)]
pub struct Armv4t;

impl Arch for Armv4t {
    type Usize = u32;
    type Registers = reg::ArmCoreRegs;

    fn target_description_xml() -> Option<&'static str> {
        Some(r#"<target version="1.0"><architecture>armv4t</architecture></target>"#)
    }
}
