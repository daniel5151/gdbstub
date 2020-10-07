use crate::arch::RegId;

/// RISC-V Register identifier.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum RiscvRegId {
    /// General Purpose Register (x0-x31).
    Gpr(u8),
    /// Floating Point Register (f0-f31).
    Fpr(u8),
    /// Program Counter.
    Pc,
    /// Control and Status Register.
    Csr(u16),
    /// Privilege level.
    Priv,
}

impl RegId for RiscvRegId {
    fn from_raw_id(id: usize) -> Option<(Self, usize)> {
        let reg_size = match id {
            0..=31 => (Self::Gpr(id as u8), 4),
            32 => (Self::Pc, 4),
            33..=64 => (Self::Fpr((id - 33) as u8), 4),
            65..=4160 => (Self::Csr((id - 65) as u16), 4),
            4161 => (Self::Priv, 1),
            _ => return None,
        };
        Some(reg_size)
    }
}
