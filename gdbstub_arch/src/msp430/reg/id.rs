use gdbstub::arch::RegId;

/// TI-MSP430 register identifier.
///
/// GDB does not provide a XML file for the MSP430.
/// The best file to reference is [msp430-tdep.c](https://github.com/bminor/binutils-gdb/blob/master/gdb/msp430-tdep.c).
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum Msp430RegId<U> {
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
    #[doc(hidden)]
    _Size(U),
}

fn from_raw_id<U>(id: usize) -> Option<Msp430RegId<U>> {
    let reg = match id {
        0 => Msp430RegId::Pc,
        1 => Msp430RegId::Sp,
        2 => Msp430RegId::Sr,
        3 => Msp430RegId::Cg,
        4..=15 => Msp430RegId::Gpr((id as u8) - 4),
        _ => return None,
    };

    Some(reg)
}

impl RegId for Msp430RegId<u16> {
    fn from_raw_id(id: usize) -> Option<Self> {
        from_raw_id::<u16>(id)
    }
}

impl RegId for Msp430RegId<u32> {
    fn from_raw_id(id: usize) -> Option<Self> {
        from_raw_id::<u32>(id)
    }
}
