use core::num::NonZeroUsize;
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
    _Size(core::marker::PhantomData<U>),
}

fn from_raw_id<U>(id: usize) -> Option<(Msp430RegId<U>, Option<NonZeroUsize>)> {
    let reg = match id {
        0 => Msp430RegId::Pc,
        1 => Msp430RegId::Sp,
        2 => Msp430RegId::Sr,
        3 => Msp430RegId::Cg,
        4..=15 => Msp430RegId::Gpr((id as u8) - 4),
        _ => return None,
    };

    let ptrsize = core::mem::size_of::<U>();
    Some((reg, Some(NonZeroUsize::new(ptrsize)?)))
}

impl RegId for Msp430RegId<u16> {
    fn from_raw_id(id: usize) -> Option<(Self, Option<NonZeroUsize>)> {
        from_raw_id::<u16>(id)
    }
}

impl RegId for Msp430RegId<u32> {
    fn from_raw_id(id: usize) -> Option<(Self, Option<NonZeroUsize>)> {
        from_raw_id::<u32>(id)
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
        serialized_data_len += RId::from_raw_id(3).unwrap().1.unwrap().get();

        // Accumulate register sizes returned by `from_raw_id`.
        let mut i = 0;
        let mut sum_reg_sizes = 0;
        while let Some((_, size)) = RId::from_raw_id(i) {
            sum_reg_sizes += size.unwrap().get();
            i += 1;
        }

        assert_eq!(serialized_data_len, sum_reg_sizes);
    }

    #[test]
    fn test_msp430() {
        test::<crate::msp430::reg::Msp430Regs<u16>, crate::msp430::reg::id::Msp430RegId<u16>>()
    }

    #[test]
    fn test_msp430x() {
        test::<crate::msp430::reg::Msp430Regs<u32>, crate::msp430::reg::id::Msp430RegId<u32>>()
    }
}
