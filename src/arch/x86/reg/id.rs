use crate::arch::RegId;

/// FPU register identifier.
#[derive(Debug, Clone, Copy)]
pub enum X87FpuInternalRegId {
    /// Floating-point control register
    Fctrl,
    /// Floating-point status register
    Fstat,
    /// Tag word
    Ftag,
    /// FPU instruction pointer segment
    Fiseg,
    /// FPU intstruction pointer offset
    Fioff,
    /// FPU operand segment
    Foseg,
    /// FPU operand offset
    Fooff,
    /// Floating-point opcode
    Fop,
}

impl X87FpuInternalRegId {
    fn from_u8(val: u8) -> Option<Self> {
        use self::X87FpuInternalRegId::*;

        let r = match val {
            0 => Fctrl,
            1 => Fstat,
            2 => Ftag,
            3 => Fiseg,
            4 => Fioff,
            5 => Foseg,
            6 => Fooff,
            7 => Fop,
            _ => return None,
        };
        Some(r)
    }
}

/// 32-bit x86 core + SSE register identifier.
///
/// Source: https://github.com/bminor/binutils-gdb/blob/master/gdb/features/i386/32bit-core.xml
/// Additionally: https://github.com/bminor/binutils-gdb/blob/master/gdb/features/i386/32bit-sse.xml
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum X86CoreRegId {
    /// Accumulator
    Eax,
    /// Count register
    Ecx,
    /// Data register
    Edx,
    /// Base register
    Ebx,
    /// Stack pointer
    Esp,
    /// Base pointer
    Ebp,
    /// Source index
    Esi,
    /// Destination index
    Edi,
    /// Instruction pointer
    Eip,
    /// Status register
    Eflags,
    /// Segment registers: CS, SS, DS, ES, FS, GS
    Segment(u8),
    /// FPU registers: ST0 through ST7
    St(u8),
    /// FPU internal registers
    Fpu(X87FpuInternalRegId),
    /// SIMD Registers: XMM0 through XMM7
    Xmm(u8),
    /// SSE Status/Control Register
    Mxcsr,
}

impl RegId for X86CoreRegId {
    fn from_raw_id(id: usize) -> Option<(Self, usize)> {
        use self::X86CoreRegId::*;

        let r = match id {
            0 => (Eax, 4),
            1 => (Ecx, 4),
            2 => (Edx, 4),
            3 => (Ebx, 4),
            4 => (Esp, 4),
            5 => (Ebp, 4),
            6 => (Esi, 4),
            7 => (Edi, 4),
            8 => (Eip, 4),
            9 => (Eflags, 4),
            10..=15 => (Segment(id as u8 - 10), 4),
            16..=23 => (St(id as u8 - 16), 10),
            24..=31 => match X87FpuInternalRegId::from_u8(id as u8 - 24) {
                Some(r) => (Fpu(r), 4),
                None => unreachable!(),
            },
            32..=39 => (Xmm(id as u8 - 32), 16),
            40 => (Mxcsr, 4),
            _ => return None,
        };
        Some(r)
    }
}

// TODO: Add proper `RegId` implementation. See [issue #29](https://github.com/daniel5151/gdbstub/issues/29)
// pub enum X86_64RegId {}

#[cfg(test)]
mod tests {
    use crate::arch::traits::RegId;
    use crate::arch::traits::Registers;

    /// Compare the following two values which are expected to be the same:
    /// * length of data written by `Registers::gdb_serialize()` in byte
    /// * sum of sizes of all registers obtained by `RegId::from_raw_id()`
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
    fn test_x86() {
        test::<crate::arch::x86::reg::X86CoreRegs, crate::arch::x86::reg::id::X86CoreRegId>()
    }
}
