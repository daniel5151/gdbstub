use crate::arch::RegId;

/// 32-bit ARM core register identifier.
#[derive(Debug, Clone, Copy)]
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
    fn from_raw_id(id: usize) -> Option<(Self, usize)> {
        match id {
            0..=12 => Some((Self::Gpr(id as u8), 4)),
            13 => Some((Self::Sp, 4)),
            14 => Some((Self::Lr, 4)),
            15 => Some((Self::Pc, 4)),
            16..=23 => Some((Self::Fpr(id as u8), 4)),
            25 => Some((Self::Cpsr, 4)),
            _ => None,
        }
    }
}
