use core::convert::TryInto;

use gdbstub::arch::Registers;

use super::{X86SegmentRegs, X87FpuInternalRegs, F80};

/// 64-bit x86 core registers (+ SSE extensions).
///
/// Source: <https://github.com/bminor/binutils-gdb/blob/master/gdb/features/i386/64bit-core.xml>
/// Additionally: <https://github.com/bminor/binutils-gdb/blob/master/gdb/features/i386/64bit-sse.xml>
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct X86_64CoreRegs {
    /// RAX, RBX, RCX, RDX, RSI, RDI, RBP, RSP, r8-r15
    pub regs: [u64; 16],
    /// Status register
    pub eflags: u32,
    /// Instruction pointer
    pub rip: u64,
    /// Segment registers: CS, SS, DS, ES, FS, GS
    pub segments: X86SegmentRegs,
    /// FPU registers: ST0 through ST7
    pub st: [F80; 8],
    /// FPU internal registers
    pub fpu: X87FpuInternalRegs,
    /// SIMD Registers: XMM0 through XMM15
    pub xmm: [u128; 0x10],
    /// SSE Status/Control Register
    pub mxcsr: u32,
}

impl Registers for X86_64CoreRegs {
    type ProgramCounter = u64;

    fn pc(&self) -> Self::ProgramCounter {
        self.rip
    }

    fn gdb_serialize(&self, mut write_byte: impl FnMut(Option<u8>)) {
        macro_rules! write_bytes {
            ($bytes:expr) => {
                for b in $bytes {
                    write_byte(Some(*b))
                }
            };
        }

        // rax, rbx, rcx, rdx, rsi, rdi, rbp, rsp, r8-r15
        for reg in &self.regs {
            write_bytes!(&reg.to_le_bytes());
        }

        // rip
        write_bytes!(&self.rip.to_le_bytes());

        // eflags
        write_bytes!(&self.eflags.to_le_bytes());

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

        // padding?
        // XXX: Couldn't figure out what these do and GDB doesn't actually display any
        // registers that use these values.
        (0..0x18).for_each(|_| write_byte(None))
    }

    fn gdb_deserialize(&mut self, bytes: &[u8]) -> Result<(), ()> {
        if bytes.len() < 0x218 {
            return Err(());
        }

        let mut regs = bytes[0..0x80]
            .chunks_exact(8)
            .map(|x| u64::from_le_bytes(x.try_into().unwrap()));

        for reg in self.regs.iter_mut() {
            *reg = regs.next().ok_or(())?;
        }

        self.rip = u64::from_le_bytes(bytes[0x80..0x88].try_into().unwrap());
        self.eflags = u32::from_le_bytes(bytes[0x88..0x8C].try_into().unwrap());

        self.segments.gdb_deserialize(&bytes[0x8C..0xA4])?;

        let mut regs = bytes[0xA4..0xF4].chunks_exact(10).map(TryInto::try_into);

        for reg in self.st.iter_mut() {
            *reg = regs.next().ok_or(())?.map_err(|_| ())?;
        }

        self.fpu.gdb_deserialize(&bytes[0xF4..0x114])?;

        let mut regs = bytes[0x114..0x214]
            .chunks_exact(0x10)
            .map(|x| u128::from_le_bytes(x.try_into().unwrap()));

        for reg in self.xmm.iter_mut() {
            *reg = regs.next().ok_or(())?;
        }

        self.mxcsr = u32::from_le_bytes(bytes[0x214..0x218].try_into().unwrap());

        Ok(())
    }
}
