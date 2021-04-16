use gdbstub::arch::RegId;

/// TI-MSP430 register identifier.
///
/// GDB does not provide a XML file for the MSP430.
/// The best file to reference is [msp430-tdep.c](https://github.com/bminor/binutils-gdb/blob/master/gdb/msp430-tdep.c).
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum Msp430RegId {
    /// Program Counter (R0)
    Pc,
    /// Stack Pointer (R1)
    Sp,
    /// Status Register (R2)
    Sr,
    /// Constant Generator (R3)
    Cg,
    /// General Purpose Registers (R4-R15)
    Gpr(u8),
}

impl RegId for Msp430RegId {
    fn from_raw_id(id: usize) -> Option<(Self, usize)> {
        let reg = match id {
            0 => Self::Pc,
            1 => Self::Sp,
            2 => Self::Sr,
            3 => Self::Cg,
            4..=15 => Self::Gpr((id as u8) - 4),
            _ => return None,
        };
        Some((reg, 2))
    }
}

#[cfg(test)]
mod tests {
    use gdbstub::arch::RegId;
    use gdbstub::arch::Registers;

    fn test<Rs: Registers, RId: RegId>() {
        // Obtain the data length written by `gdb_serialize` by passing a custom
        // closure.
        let mut serialized_data_len = 0;
        let counter = |b: Option<u8>| {
            if b.is_some() {
                serialized_data_len += 1;
            }
        };
        Rs::default().gdb_serialize(counter);

        // The `Msp430Regs` implementation does not increment the size for
        // the CG register since it will always be the constant zero.
        serialized_data_len += 4;

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
    fn test_msp430() {
        test::<crate::msp430::reg::Msp430Regs, crate::msp430::reg::id::Msp430RegId>()
    }
}
