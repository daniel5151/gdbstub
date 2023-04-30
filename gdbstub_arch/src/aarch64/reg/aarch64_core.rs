use core::convert::TryInto;
use gdbstub::arch::Registers;

/// AArch64 core registers.
///
/// Registers from the `org.gnu.gdb.aarch64.core` and `org.gnu.gdb.aarch64.fpu`
/// [AArch64 Standard GDB Target Features](https://sourceware.org/gdb/onlinedocs/gdb/AArch64-Features.html).
#[derive(Debug, Default, Clone, Eq, PartialEq)]
pub struct AArch64CoreRegs {
    /// General Purpose Registers (X0-X30)
    pub x: [u64; 31],
    /// Stack Pointer
    pub sp: u64,
    /// Program Counter
    pub pc: u64,
    /// Process State (GDB uses the AArch32 CPSR name)
    pub cpsr: u32,
    /// FP & SIMD Registers (V0-V31)
    pub v: [u128; 32],
    /// Floating-point Control Register
    pub fpcr: u32,
    /// Floating-point Status Register
    pub fpsr: u32,
}

impl Registers for AArch64CoreRegs {
    type ProgramCounter = u64;

    fn pc(&self) -> Self::ProgramCounter {
        self.pc
    }

    fn gdb_serialize(&self, mut write_byte: impl FnMut(Option<u8>)) {
        macro_rules! write_bytes {
            ($var: expr) => {
                for b in $var.to_le_bytes() {
                    write_byte(Some(b))
                }
            };
        }

        for reg in self.x.iter() {
            write_bytes!(reg);
        }
        write_bytes!(self.sp);
        write_bytes!(self.pc);
        write_bytes!(self.cpsr);
        for reg in self.v.iter() {
            write_bytes!(reg);
        }
        write_bytes!(self.fpcr);
        write_bytes!(self.fpsr);
    }

    fn gdb_deserialize(&mut self, bytes: &[u8]) -> Result<(), ()> {
        const CPSR_OFF: usize = core::mem::size_of::<u64>() * 33;
        const FPSIMD_OFF: usize = CPSR_OFF + core::mem::size_of::<u32>();
        const FPCR_OFF: usize = FPSIMD_OFF + core::mem::size_of::<u128>() * 32;
        const END: usize = FPCR_OFF + core::mem::size_of::<u32>() * 2;

        if bytes.len() < END {
            return Err(());
        }

        let mut regs = bytes[0..CPSR_OFF]
            .chunks_exact(core::mem::size_of::<u64>())
            .map(|c| u64::from_le_bytes(c.try_into().unwrap()));

        for reg in self.x.iter_mut() {
            *reg = regs.next().ok_or(())?
        }
        self.sp = regs.next().ok_or(())?;
        self.pc = regs.next().ok_or(())?;

        let mut regs = bytes[CPSR_OFF..FPSIMD_OFF]
            .chunks_exact(core::mem::size_of::<u32>())
            .map(|c| u32::from_le_bytes(c.try_into().unwrap()));

        self.cpsr = regs.next().ok_or(())?;

        let mut regs = bytes[FPSIMD_OFF..FPCR_OFF]
            .chunks_exact(core::mem::size_of::<u128>())
            .map(|c| u128::from_le_bytes(c.try_into().unwrap()));

        for reg in self.v.iter_mut() {
            *reg = regs.next().ok_or(())?
        }

        let mut regs = bytes[FPCR_OFF..]
            .chunks_exact(core::mem::size_of::<u32>())
            .map(|c| u32::from_le_bytes(c.try_into().unwrap()));

        self.fpcr = regs.next().ok_or(())?;
        self.fpsr = regs.next().ok_or(())?;

        Ok(())
    }
}
