use crate::arch::x86::reg::F80;
use crate::arch::Registers;
use core::convert::TryInto;

/// 64-bit x86 core registers.
///
/// Source: https://github.com/bminor/binutils-gdb/blob/master/gdb/features/i386/64bit-core.xml
#[derive(Default)]
pub struct X86_64CoreRegs {
    /// RAX, RBX, RCX, RDX, RSI, RDI, RBP, RSP, r8-r15
    pub regs: [u64; 16],
    /// Status register
    pub eflags: u32,
    /// Instruction pointer
    pub rip: u64,
    /// Segment registers: CS, SS, DS, ES, FS, GS
    pub segments: [u32; 6],
    /// FPU registers: ST0 through ST7
    pub st: [F80; 8],
    /// Floating-point control register
    pub fctrl: u32,
    /// Floating-point status register
    pub fstat: u32,
    /// Tag word
    pub ftag: u32,
    /// FPU instruction pointer segment
    pub fiseg: u32,
    /// FPU intstruction pointer offset
    pub fioff: u32,
    /// FPU operand segment
    pub foseg: u32,
    /// FPU operand offset
    pub fooff: u32,
    /// Floating-point opcode
    pub fop: u32,
    /// SIMD Registers: XMM0 through XMM15
    pub xmm: [u128; 0x10],
    /// SSE Status/Control Register
    pub mxcsr: u32,
}

impl Registers for X86_64CoreRegs {
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
        for st_reg in &self.st {
            write_bytes!(st_reg);
        }

        // Note: GDB section names don't make sense unless you read x87 FPU section 8.1:
        // https://web.archive.org/web/20150123212110/http://www.intel.com/content/dam/www/public/us/en/documents/manuals/64-ia-32-architectures-software-developer-vol-1-manual.pdf
        write_bytes!(&self.fctrl.to_le_bytes());
        write_bytes!(&self.fstat.to_le_bytes());
        write_bytes!(&self.ftag.to_le_bytes());
        write_bytes!(&self.fiseg.to_le_bytes());
        write_bytes!(&self.fioff.to_le_bytes());
        write_bytes!(&self.foseg.to_le_bytes());
        write_bytes!(&self.fooff.to_le_bytes());
        write_bytes!(&self.fop.to_le_bytes());

        // xmm0 to xmm15
        for xmm_reg in &self.xmm {
            write_bytes!(&xmm_reg.to_le_bytes());
        }

        // mxcsr
        write_bytes!(&self.mxcsr.to_le_bytes());

        // padding?
        write_bytes!(&[0; 0x18]);
    }

    fn gdb_deserialize(&mut self, bytes: &[u8]) -> Result<(), ()> {
        let mut regs = bytes[0..0x80].chunks_exact(8).map(|x| x.try_into());

        for reg in self.regs.iter_mut() {
            *reg = u64::from_le_bytes(regs.next().ok_or(())?.map_err(|_| ())?);
        }

        self.rip = u64::from_le_bytes(bytes[0x80..0x88].try_into().map_err(|_| ())?);
        self.eflags = u32::from_le_bytes(bytes[0x88..0x8C].try_into().map_err(|_| ())?);

        let mut segments = bytes[0x8C..0xA4].chunks_exact(4).map(|x| x.try_into());

        for seg in self.segments.iter_mut() {
            *seg = u32::from_le_bytes(segments.next().ok_or(())?.map_err(|_| ())?);
        }

        let mut regs = bytes[0xA4..0xF4].chunks_exact(10).map(TryInto::try_into);

        for reg in self.st.iter_mut() {
            *reg = regs.next().ok_or(())?.map_err(|_| ())?;
        }

        let mut regs = bytes[0xF4..0x114]
            .chunks_exact(4)
            .map(|x| u32::from_le_bytes(x.try_into().unwrap()));

        self.fctrl = regs.next().ok_or(())?;
        self.fstat = regs.next().ok_or(())?;
        self.ftag = regs.next().ok_or(())?;
        self.fiseg = regs.next().ok_or(())?;
        self.fioff = regs.next().ok_or(())?;
        self.foseg = regs.next().ok_or(())?;
        self.fooff = regs.next().ok_or(())?;
        self.fop = regs.next().ok_or(())?;

        let mut regs = bytes[0x114..0x214]
            .chunks_exact(0x10)
            .map(TryInto::try_into);

        for reg in self.xmm.iter_mut() {
            *reg = u128::from_le_bytes(regs.next().ok_or(())?.map_err(|_| ())?);
        }

        self.mxcsr = u32::from_le_bytes(bytes[0x214..0x218].try_into().map_err(|_| ())?);

        Ok(())
    }
}