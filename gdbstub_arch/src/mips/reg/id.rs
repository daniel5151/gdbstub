use gdbstub::arch::RegId;

/// MIPS register identifier.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum MipsRegId<U> {
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
    #[doc(hidden)]
    _Size(U),
}

fn from_raw_id<U>(id: usize) -> Option<(MipsRegId<U>, usize)> {
    let reg = match id {
        0..=31 => MipsRegId::Gpr(id as u8),
        32 => MipsRegId::Status,
        33 => MipsRegId::Lo,
        34 => MipsRegId::Hi,
        35 => MipsRegId::Badvaddr,
        36 => MipsRegId::Cause,
        37 => MipsRegId::Pc,
        38..=69 => MipsRegId::Fpr((id as u8) - 38),
        70 => MipsRegId::Fcsr,
        71 => MipsRegId::Fir,
        72 => MipsRegId::Hi1,
        73 => MipsRegId::Lo1,
        74 => MipsRegId::Hi2,
        75 => MipsRegId::Lo2,
        76 => MipsRegId::Hi3,
        77 => MipsRegId::Lo3,
        // `MipsRegId::Dspctl` is the only register that will always be 4 bytes wide
        78 => return Some((MipsRegId::Dspctl, 4)),
        79 => MipsRegId::Restart,
        _ => return None,
    };

    let ptrsize = core::mem::size_of::<U>();
    Some((reg, ptrsize))
}

impl RegId for MipsRegId<u32> {
    fn from_raw_id(id: usize) -> Option<(Self, usize)> {
        from_raw_id::<u32>(id)
    }
}

impl RegId for MipsRegId<u64> {
    fn from_raw_id(id: usize) -> Option<(Self, usize)> {
        from_raw_id::<u64>(id)
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
        test::<crate::mips::reg::MipsCoreRegsWithDsp<u32>, crate::mips::reg::id::MipsRegId<u32>>()
    }

    #[test]
    fn test_mips64() {
        test::<crate::mips::reg::MipsCoreRegsWithDsp<u64>, crate::mips::reg::id::MipsRegId<u64>>()
    }
}
