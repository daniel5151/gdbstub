use crate::arch::RegId;

/// 32-bit MIPS register identifier.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum MipsRegId {
    /// General purpose registers (R0-R31)
    Gpr(u8),
    /// Status register
    Status,
    /// Low register
    Lo,
    /// High register
    Hi,
    /// Bad Virtual Address register
    Badvaddr,
    /// Exception Cause register
    Cause,
    /// Program Counter
    Pc,
    /// Floating point registers (F0-F31)
    Fpr(u8),
    /// Floating-point Control Status register
    Fcsr,
    /// Floating-point Implementation Register
    Fir,
    /// High 1 register
    Hi1,
    /// Low 1 register
    Lo1,
    /// High 2 register
    Hi2,
    /// Low 2 register
    Lo2,
    /// High 3 register
    Hi3,
    /// Low 3 register
    Lo3,
    /// DSP Control register
    Dspctl,
    /// Restart register
    Restart,
}

/// 64-bit MIPS register identifier.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum Mips64RegId {
    /// General purpose registers (R0-R31)
    Gpr(u8),
    /// Status register
    Status,
    /// Low register
    Lo,
    /// High register
    Hi,
    /// Bad Virtual Address register
    Badvaddr,
    /// Exception Cause register
    Cause,
    /// Program Counter
    Pc,
    /// Floating point registers (F0-F31)
    Fpr(u8),
    /// Floating-point Control Status register
    Fcsr,
    /// Floating-point Implementation Register
    Fir,
    /// High 1 register
    Hi1,
    /// Low 1 register
    Lo1,
    /// High 2 register
    Hi2,
    /// Low 2 register
    Lo2,
    /// High 3 register
    Hi3,
    /// Low 3 register
    Lo3,
    /// DSP Control register
    Dspctl,
    /// Restart register
    Restart,
}

impl RegId for MipsRegId {
    fn from_raw_id(id: usize) -> Option<(Self, usize)> {
        let reg = match id {
            0..=31 => Self::Gpr(id as u8),
            32 => Self::Status,
            33 => Self::Lo,
            34 => Self::Hi,
            35 => Self::Badvaddr,
            36 => Self::Cause,
            37 => Self::Pc,
            38..=69 => Self::Fpr((id as u8) - 38),
            70 => Self::Fcsr,
            71 => Self::Fir,
            72 => Self::Hi1,
            73 => Self::Lo1,
            74 => Self::Hi2,
            75 => Self::Lo2,
            76 => Self::Hi3,
            77 => Self::Lo3,
            78 => Self::Dspctl,
            79 => Self::Restart,
            _ => return None,
        };
        Some((reg, 4))
    }
}

impl RegId for Mips64RegId {
    fn from_raw_id(id: usize) -> Option<(Self, usize)> {
        let reg = match id {
            0..=31 => Self::Gpr(id as u8),
            32 => Self::Status,
            33 => Self::Lo,
            34 => Self::Hi,
            35 => Self::Badvaddr,
            36 => Self::Cause,
            37 => Self::Pc,
            38..=69 => Self::Fpr((id as u8) - 38),
            70 => Self::Fcsr,
            71 => Self::Fir,
            72 => Self::Hi1,
            73 => Self::Lo1,
            74 => Self::Hi2,
            75 => Self::Lo2,
            76 => Self::Hi3,
            77 => Self::Lo3,
            78 => Self::Dspctl,
            79 => Self::Restart,
            _ => return None,
        };
        Some((reg, 8))
    }
}

#[cfg(test)]
mod tests {
    use crate::arch::traits::RegId;
    use crate::arch::traits::Registers;

    fn test<Rs: Registers, RId: RegId>() {
        // Obtain the data length written by `gdb_serialize` by passing a custom closure.
        let mut serialized_data_len = 0;
        let counter = |b: Option<u8>| {
            if b.is_some() {
                serialized_data_len += 1;
            }
        };
        Rs::default().gdb_serialize(counter);

        // Accumulate register sizes returned by `from_raw_id`.
        let mut i = 0;
        let mut sum_reg_sizes = 0;
        while let Some((_, size)) = RId::from_raw_id(i) {
            sum_reg_sizes += size;
            i += 1;
        }

        assert_eq!(serialized_data_len, sum_reg_sizes);
    }

    #[test]
    fn test_mips32() {
        test::<crate::arch::mips::reg::MipsCoreRegs<u32>, crate::arch::mips::reg::id::MipsRegId>()
    }

    #[test]
    fn test_mips64() {
        test::<crate::arch::mips::reg::MipsCoreRegs<u64>, crate::arch::mips::reg::id::Mips64RegId>()
    }
}
