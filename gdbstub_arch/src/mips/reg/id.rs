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

fn from_raw_id<U>(id: usize) -> Option<MipsRegId<U>> {
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
        78 => MipsRegId::Dspctl,
        79 => MipsRegId::Restart,
        _ => return None,
    };

    Some(reg)
}

impl RegId for MipsRegId<u32> {
    fn from_raw_id(id: usize) -> Option<Self> {
        from_raw_id::<u32>(id)
    }
}

impl RegId for MipsRegId<u64> {
    fn from_raw_id(id: usize) -> Option<Self> {
        from_raw_id::<u64>(id)
    }
}
