use crate::arch::Registers;

/// 64-bit MIPS registers.
///
/// Source: https://github.com/bminor/binutils-gdb/blob/master/gdb/features/mips64-cpu.xml
#[derive(Default)]
pub struct Mips64CoreRegs {
    /// General purpose registers (R0-R32)
    pub r: [u64; 32],
    /// Low register (regnum 33)
    pub lo: u64,
    /// High register (regnum 34)
    pub hi: u64,
    /// Program Counter (regnum 37)
    pub pc: u64,
    /// CP0 registers
    pub cp0: Mips64Cp0Regs,
    /// FPU registers
    pub fpu: Mips64FpuRegs,
}

/// 64-bit MIPS CP0 (coprocessor 0) registers.
///
/// Source: https://github.com/bminor/binutils-gdb/blob/master/gdb/features/mips64-cp0.xml
#[derive(Default)]
pub struct Mips64Cp0Regs {
    /// Status register (regnum 32)
    pub status: u64,
    /// Bad Virtual Address register (regnum 35)
    pub badvaddr: u64,
    /// Exception Cause register (regnum 36)
    pub cause: u64,
}

/// 64-bit MIPS FPU registers.
///
/// Source: https://github.com/bminor/binutils-gdb/blob/master/gdb/features/mips64-fpu.xml
#[derive(Default)]
pub struct Mips64FpuRegs {
    /// FP registers (F0-F32) starting at regnum 38
    pub r: [u64; 32],
    /// Floating-point Control Status register
    pub fcsr: u64,
    /// Floating-point Implementation Register
    pub fir: u64,
}

impl Registers for Mips64CoreRegs {
    fn gdb_serialize(&self, mut write_byte: impl FnMut(Option<u8>)) {
        macro_rules! write_bytes {
            ($bytes:expr) => {
                for b in $bytes {
                    write_byte(Some(*b))
                }
            };
        }

        // Write GPRs
        for reg in self.r.iter() {
            write_bytes!(&reg.to_le_bytes());
        }

        // Status register is regnum 32
        write_bytes!(&self.cp0.status.to_le_bytes());

        // Low and high registers are regnums 33 and 34
        write_bytes!(&self.lo.to_le_bytes());
        write_bytes!(&self.hi.to_le_bytes());

        // Badvaddr and Cause registers are regnums 35 and 36
        write_bytes!(&self.cp0.badvaddr.to_le_bytes());
        write_bytes!(&self.cp0.cause.to_le_bytes());

        // Program Counter is regnum 37
        write_bytes!(&self.pc.to_le_bytes());

        // Write FPRs
        for reg in self.fpu.r.iter() {
            write_bytes!(&reg.to_le_bytes());
        }

        // Write FCSR and FIR registers
        write_bytes!(&self.fpu.fcsr.to_le_bytes());
        write_bytes!(&self.fpu.fir.to_le_bytes());
    }

    fn gdb_deserialize(&mut self, bytes: &[u8]) -> Result<(), ()> {
        // ensure bytes.chunks_exact(4) won't panic
        if bytes.len() % 8 != 0 {
            return Err(());
        }

        use core::convert::TryInto;
        let mut regs = bytes
            .chunks_exact(8)
            .map(|c| u64::from_le_bytes(c.try_into().unwrap()));

        // Read GPRs
        for reg in self.r.iter_mut() {
            *reg = regs.next().ok_or(())?
        }

        // Read Status register
        self.cp0.status = regs.next().ok_or(())?;

        // Read Low and High registers
        self.lo = regs.next().ok_or(())?;
        self.hi = regs.next().ok_or(())?;

        // Read Badvaddr and Cause registers
        self.cp0.badvaddr = regs.next().ok_or(())?;
        self.cp0.cause = regs.next().ok_or(())?;

        // Read the Program Counter
        self.pc = regs.next().ok_or(())?;

        // Read FPRs
        for reg in self.fpu.r.iter_mut() {
            *reg = regs.next().ok_or(())?
        }

        // Read FCSR and FIR registers
        self.fpu.fcsr = regs.next().ok_or(())?;
        self.fpu.fir = regs.next().ok_or(())?;

        if regs.next().is_some() {
            return Err(());
        }

        Ok(())
    }
}
