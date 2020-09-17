use crate::arch::RegId;

/// RISC-V Register identifier.
#[derive(Debug, Clone, Copy)]
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
        match id {
            0..=31 => Some((Self::Gpr(id as u8), 4)),
            32 => Some((Self::Pc, 4)),
            33..=64 => Some((Self::Fpr((id - 33) as u8), 4)),
            65..=4160 => Some((Self::Csr((id - 65) as u16), 4)),
            4161 => Some((Self::Priv, 1)),
            _ => None,
        }
    }
}
