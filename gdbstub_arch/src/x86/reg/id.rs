use gdbstub::arch::RegId;

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
    /// FPU instruction pointer offset
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

/// Segment register identifier.
#[derive(Debug, Clone, Copy)]
#[allow(clippy::upper_case_acronyms)]
pub enum X86SegmentRegId {
    /// Code Segment
    CS,
    /// Stack Segment
    SS,
    /// Data Segment
    DS,
    /// Extra Segment
    ES,
    /// General Purpose Segment
    FS,
    /// General Purpose Segment
    GS,
}

impl X86SegmentRegId {
    fn from_u8(val: u8) -> Option<Self> {
        use self::X86SegmentRegId::*;

        let r = match val {
            0 => CS,
            1 => SS,
            2 => DS,
            3 => ES,
            4 => FS,
            5 => GS,
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
    /// Segment registers
    Segment(X86SegmentRegId),
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
    fn from_raw_id(id: usize) -> Option<Self> {
        use self::X86CoreRegId::*;

        let r = match id {
            0 => Eax,
            1 => Ecx,
            2 => Edx,
            3 => Ebx,
            4 => Esp,
            5 => Ebp,
            6 => Esi,
            7 => Edi,
            8 => Eip,
            9 => Eflags,
            10..=15 => Segment(X86SegmentRegId::from_u8(id as u8 - 10)?),
            16..=23 => St(id as u8 - 16),
            24..=31 => Fpu(X87FpuInternalRegId::from_u8(id as u8 - 24)?),
            32..=39 => Xmm(id as u8 - 32),
            40 => Mxcsr,
            _ => return None,
        };
        Some(r)
    }
}

/// 64-bit x86 core + SSE register identifier.
///
/// Source: https://github.com/bminor/binutils-gdb/blob/master/gdb/features/i386/64bit-core.xml
/// Additionally: https://github.com/bminor/binutils-gdb/blob/master/gdb/features/i386/64bit-sse.xml
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum X86_64CoreRegId {
    /// General purpose registers:
    /// RAX, RBX, RCX, RDX, RSI, RDI, RBP, RSP, r8-r15
    Gpr(u8),
    /// Instruction pointer
    Rip,
    /// Status register
    Eflags,
    /// Segment registers
    Segment(X86SegmentRegId),
    /// FPU registers: ST0 through ST7
    St(u8),
    /// FPU internal registers
    Fpu(X87FpuInternalRegId),
    /// SIMD Registers: XMM0 through XMM15
    Xmm(u8),
    /// SSE Status/Control Register
    Mxcsr,
}

impl RegId for X86_64CoreRegId {
    fn from_raw_id(id: usize) -> Option<Self> {
        use self::X86_64CoreRegId::*;

        let r = match id {
            0..=15 => Gpr(id as u8),
            16 => Rip,
            17 => Eflags,
            18..=23 => Segment(X86SegmentRegId::from_u8(id as u8 - 18)?),
            24..=31 => St(id as u8 - 24),
            32..=39 => Fpu(X87FpuInternalRegId::from_u8(id as u8 - 32)?),
            40..=55 => Xmm(id as u8 - 40),
            56 => Mxcsr,
            _ => return None,
        };
        Some(r)
    }
}
