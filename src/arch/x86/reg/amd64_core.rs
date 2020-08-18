use crate::arch::Registers;
use core::convert::TryInto;

/// 64-bit x86 core registers.
///
/// Source: https://github.com/bminor/binutils-gdb/blob/master/gdb/features/arm/arm-core.xml
#[derive(Default)]
pub struct Amd64CoreRegs {
    // RAX, RBX, RCX, RDX, RSI, RDI, RBP, RSP, r8-r15
    regs: [u64; 16],
    // Status register
    eflags: u32,
    // Instruction pointer
    rip: u64,
    // Segment registers: CS, SS, DS, ES, FS, GS
    segments: [u32; 6],
    // FPU registers: ST0 through ST7
    st_regs: [F80; 8],
    // SIMD Registers: XMM0 through XMM15
    xmm_regs: [u128; 0x10],
    // SSE Status/Control Register
    mxcsr: u32,
}

type F80 = [u8; 10];

impl Registers for Amd64CoreRegs {
    fn gdb_serialize(&self, mut write_byte: impl FnMut(Option<u8>)) {
        macro_rules! write_bytes {
            ($bytes:expr) => {
                for b in $bytes {
                    write_byte(Some(*b))
                }
            };
        }

        for reg in &self.regs {
            write_bytes!(&reg.to_le_bytes());
        }

        // rip
        write_bytes!(&self.rip.to_le_bytes());

        // eflags
        write_bytes!(&self.eflags.to_le_bytes());

        // cs, ss, ds, es, fs, gs
        for seg in &self.segments {
            write_bytes!(&seg.to_le_bytes());
        }

        // st0 to st7
        for st_reg in &self.st_regs {
            write_bytes!(st_reg);
        }

        // fctrl, fstat, ftag, fiseg, fioff, foseg, fooff, fop
        for _ in 0..8 {
            write_bytes!(&[0; 4]);
        }

        // xmm0 to xmm15
        for xmm_reg in &self.xmm_regs {
            write_bytes!(&xmm_reg.to_le_bytes());
        }

        // mxcsr
        write_bytes!(&self.mxcsr.to_le_bytes());

        // padding?
        write_bytes!(&[0x10; 0x18]);
    }

    fn gdb_deserialize(&mut self, bytes: &[u8]) -> Result<(), ()> {
        let mut regs = bytes[0..0x80]
            .chunks_exact(8)
            .map(|x| u64::from_le_bytes(x.try_into().unwrap()));

        for reg in self.regs.iter_mut() {
            *reg = regs.next().unwrap();
        }

        self.rip = u64::from_le_bytes(bytes[0x80..0x88].try_into().unwrap());
        self.eflags = u32::from_le_bytes(bytes[0x88..0x8C].try_into().unwrap());

        let mut segments = bytes[0x8C..0xA4]
            .chunks_exact(4)
            .map(|x| u32::from_le_bytes(x.try_into().unwrap()));

        for seg in self.segments.iter_mut() {
            *seg = segments.next().unwrap();
        }

        let mut regs = bytes[0xA4..0xF4]
            .chunks_exact(10)
            .map(|x| x.try_into().unwrap());

        for reg in self.st_regs.iter_mut() {
            *reg = regs.next().unwrap();
        }

        let mut regs = bytes[0x114..0x214]
            .chunks_exact(0x10)
            .map(|x| u128::from_le_bytes(x.try_into().unwrap()));

        for reg in self.xmm_regs.iter_mut() {
            *reg = regs.next().unwrap();
        }

        self.mxcsr = u32::from_le_bytes(bytes[0x214..0x218].try_into().unwrap());

        Ok(())
    }
}

// rax - 0
// rbx - 8
// rcx - 0x10
// rdx - 0x18
// rsi - 0x20
// rdi - 0x28
// rbp - 0x30
// rsp - 0x38
// r8 - 0x40
// ...
// r15 - 0x78
// rip - 0x80
// eflags - 0x88
// cs 0x8c
// ss 0x90
// ds 0x94
// es 0x98
// fs 0x9c
// gs 0xa0
// st0 0xa4
// st1 0xae
// ...
// st7 0xea
// fctrl 0xf4
// fstat f8
// ftag fc
// fiseg 100
// fioff 104
// foseg 108
// fooff 10c
// fop 110
// xmm0 - 0x114
// xmm1 - 0x124
// xmm2 - 0x134
// xmm3 - 0x144
// xmm4 - 0x154
// ...
// xmm15 - 0x204
// mxcsr - 0x214
// padding??? - 0x218
