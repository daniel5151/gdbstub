use core::num::NonZeroUsize;

use gdbstub::arch::RegId;

/// 32-bit ARM core register identifier.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum ArmCoreRegId {
    /// General purpose registers (R0-R12)
    Gpr(u8),
    /// Stack Pointer (R13)
    Sp,
    /// Link Register (R14)
    Lr,
    /// Program Counter (R15)
    Pc,
    /// Floating point registers (F0-F7)
    Fpr(u8),
    /// Floating point status
    Fps,
    /// Current Program Status Register (cpsr)
    Cpsr,
}

impl RegId for ArmCoreRegId {
    fn from_raw_id(id: usize) -> Option<(Self, Option<NonZeroUsize>)> {
        let reg = match id {
            0..=12 => Self::Gpr(id as u8),
            13 => Self::Sp,
            14 => Self::Lr,
            15 => Self::Pc,
            16..=23 => Self::Fpr((id as u8) - 16),
            25 => Self::Cpsr,
            _ => return None,
        };
        Some((reg, Some(NonZeroUsize::new(4)?)))
    }
}
