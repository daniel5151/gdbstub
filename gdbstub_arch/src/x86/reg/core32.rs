use super::X86SegmentRegs;
use super::X87FpuInternalRegs;
use super::F80;
use core::convert::TryInto;
use gdbstub::arch::Registers;

/// 32-bit x86 core registers (+ SSE extensions).
///
/// Source: <https://github.com/bminor/binutils-gdb/blob/master/gdb/features/i386/32bit-core.xml>
/// Additionally: <https://github.com/bminor/binutils-gdb/blob/master/gdb/features/i386/32bit-sse.xml>
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct X86CoreRegs {
    /// Accumulator
    pub eax: u32,
    /// Count register
    pub ecx: u32,
    /// Data register
    pub edx: u32,
    /// Base register
    pub ebx: u32,
    /// Stack pointer
    pub esp: u32,
    /// Base pointer
    pub ebp: u32,
    /// Source index
    pub esi: u32,
    /// Destination index
    pub edi: u32,
    /// Instruction pointer
    pub eip: u32,
    /// Status register
    pub eflags: u32,
    /// Segment registers: CS, SS, DS, ES, FS, GS
    pub segments: X86SegmentRegs,
    /// FPU registers: ST0 through ST7
    pub st: [F80; 8],
    /// FPU internal registers
    pub fpu: X87FpuInternalRegs,
    /// SIMD Registers: XMM0 through XMM7
    pub xmm: [u128; 8],
    /// SSE Status/Control Register
    pub mxcsr: u32,
}

impl Registers for X86CoreRegs {
    type ProgramCounter = u32;

    fn pc(&self) -> Self::ProgramCounter {
        self.eip
    }

    fn gdb_serialize(&self, mut write_byte: impl FnMut(Option<u8>)) {
        macro_rules! write_bytes {
            ($bytes:expr) => {
                for b in $bytes {
                    write_byte(Some(*b))
                }
            };
        }

        macro_rules! write_regs {
            ($($reg:ident),*) => {
                $(
                    write_bytes!(&self.$reg.to_le_bytes());
                )*
            }
        }

        write_regs!(eax, ecx, edx, ebx, esp, ebp, esi, edi, eip, eflags);

        self.segments.gdb_serialize(&mut write_byte);

        // st0 to st7
        for st_reg in &self.st {
            write_bytes!(st_reg);
        }

        self.fpu.gdb_serialize(&mut write_byte);

        // xmm0 to xmm15
        for xmm_reg in &self.xmm {
            write_bytes!(&xmm_reg.to_le_bytes());
        }

        // mxcsr
        write_bytes!(&self.mxcsr.to_le_bytes());

        // padding
        (0..4).for_each(|_| write_byte(None))
    }

    fn gdb_deserialize(&mut self, bytes: &[u8]) -> Result<(), ()> {
        if bytes.len() < 0x138 {
            return Err(());
        }

        macro_rules! parse_regs {
            ($($reg:ident),*) => {
                let mut regs = bytes[0..0x28]
                    .chunks_exact(4)
                    .map(|x| u32::from_le_bytes(x.try_into().unwrap()));
                $(
                    self.$reg = regs.next().ok_or(())?;
                )*
            }
        }

        parse_regs!(eax, ecx, edx, ebx, esp, ebp, esi, edi, eip, eflags);

        self.segments.gdb_deserialize(&bytes[0x28..0x40])?;

        let mut regs = bytes[0x40..0x90].chunks_exact(10).map(TryInto::try_into);

        for reg in self.st.iter_mut() {
            *reg = regs.next().ok_or(())?.map_err(|_| ())?;
        }

        self.fpu.gdb_deserialize(&bytes[0x90..0xb0])?;

        let mut regs = bytes[0xb0..0x130]
            .chunks_exact(0x10)
            .map(|x| u128::from_le_bytes(x.try_into().unwrap()));

        for reg in self.xmm.iter_mut() {
            *reg = regs.next().ok_or(())?;
        }

        self.mxcsr = u32::from_le_bytes(bytes[0x130..0x134].try_into().unwrap());

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn x86_core_round_trip() {
        let regs_before = X86CoreRegs {
            eax: 1,
            ecx: 2,
            edx: 3,
            ebx: 4,
            esp: 5,
            ebp: 6,
            esi: 7,
            edi: 8,
            eip: 9,
            eflags: 10,
            segments: X86SegmentRegs {
                cs: 11,
                ss: 12,
                ds: 13,
                es: 14,
                fs: 15,
                gs: 16,
            },
            st: Default::default(),
            fpu: X87FpuInternalRegs {
                fctrl: 17,
                fstat: 18,
                ftag: 19,
                fiseg: 20,
                fioff: 21,
                foseg: 22,
                fooff: 23,
                fop: 24,
            },
            xmm: Default::default(),
            mxcsr: 99,
        };

        let mut data = vec![];

        regs_before.gdb_serialize(|x| {
            data.push(x.unwrap_or(b'x'));
        });

        let mut regs_after = X86CoreRegs::default();
        regs_after.gdb_deserialize(&data).unwrap();

        assert_eq!(regs_before, regs_after);
    }
}
