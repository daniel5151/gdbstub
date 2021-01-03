use crate::arch::RegId;

/// 32-bit PowerPC register identifier.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum PowerPc32RegId {
    /// General purpose registers (R0-R31)
    Gpr(u8),
    /// Floating point registers (F0-F31)
    Fpr(u8),
    /// Program Counter
    Pc,
    /// Machine state
    Msr,
    /// Condition register
    Cr,
    /// Link register
    Lr,
    /// Count register
    Ctr,
    /// Integer exception register
    Xer,
    /// Floating-point status and control register
    Fpscr,
    /// Vector registers
    Vr(u8),
    /// Vector status and control register
    Vscr,
    /// Vector context save register
    Vrsave,
}

impl RegId for PowerPc32RegId {
    fn from_raw_id(id: usize) -> Option<(Self, usize)> {
        let reg = match id {
            0..=31 => (Self::Gpr(id as u8), 4),
            32..=63 => (Self::Fpr((id as u8) - 32), 8),
            64 => (Self::Pc, 4),
            65 => (Self::Msr, 4),
            66 => (Self::Cr, 4),
            67 => (Self::Lr, 4),
            68 => (Self::Ctr, 4),
            69 => (Self::Xer, 4),
            70 => (Self::Fpscr, 4),
            71..=102 => (Self::Vr((id as u8) - 71), 16),
            103 => (Self::Vscr, 4),
            104 => (Self::Vrsave, 4),
            _ => return None,
        };

        Some(reg)
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
    fn test_powerpc() {
        test::<crate::arch::ppc::reg::PowerPcCommonRegs, crate::arch::ppc::reg::id::PowerPc32RegId>()
    }
}
