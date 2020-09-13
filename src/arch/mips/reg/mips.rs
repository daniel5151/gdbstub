use crate::arch::RawRegId;
use crate::arch::Registers;
use crate::internal::LeBytes;

use num_traits::PrimInt;

/// MIPS registers.
///
/// The register width is set to `u32` or `u64` based on the `<U>` type.
///
/// Source: https://github.com/bminor/binutils-gdb/blob/master/gdb/features/mips-cpu.xml
#[derive(Debug, Default, Clone, Eq, PartialEq)]
pub struct MipsCoreRegs<U> {
    /// General purpose registers (R0-R32)
    pub r: [U; 32],
    /// Low register (regnum 33)
    pub lo: U,
    /// High register (regnum 34)
    pub hi: U,
    /// Program Counter (regnum 37)
    pub pc: U,
    /// CP0 registers
    pub cp0: MipsCp0Regs<U>,
    /// FPU registers
    pub fpu: MipsFpuRegs<U>,
}

/// MIPS CP0 (coprocessor 0) registers.
///
/// Source: https://github.com/bminor/binutils-gdb/blob/master/gdb/features/mips-cp0.xml
#[derive(Debug, Default, Clone, Eq, PartialEq)]
pub struct MipsCp0Regs<U> {
    /// Status register (regnum 32)
    pub status: U,
    /// Bad Virtual Address register (regnum 35)
    pub badvaddr: U,
    /// Exception Cause register (regnum 36)
    pub cause: U,
}

/// MIPS FPU registers.
///
/// Source: https://github.com/bminor/binutils-gdb/blob/master/gdb/features/mips-fpu.xml
#[derive(Debug, Default, Clone, Eq, PartialEq)]
pub struct MipsFpuRegs<U> {
    /// FP registers (F0-F32) starting at regnum 38
    pub r: [U; 32],
    /// Floating-point Control Status register
    pub fcsr: U,
    /// Floating-point Implementation Register
    pub fir: U,
}

impl<U> Registers for MipsCoreRegs<U>
where
    U: PrimInt + LeBytes + Default + core::fmt::Debug,
{
    type RegId = RawRegId;

    fn gdb_serialize(&self, mut write_byte: impl FnMut(Option<u8>)) {
        macro_rules! write_le_bytes {
            ($value:expr) => {
                let mut buf = [0; 16];
                // infallible (unless digit is a >128 bit number)
                let len = $value.to_le_bytes(&mut buf).unwrap();
                let buf = &buf[..len];
                for b in buf {
                    write_byte(Some(*b));
                }
            };
        }

        // Write GPRs
        for reg in self.r.iter() {
            write_le_bytes!(reg);
        }

        // Status register is regnum 32
        write_le_bytes!(&self.cp0.status);

        // Low and high registers are regnums 33 and 34
        write_le_bytes!(&self.lo);
        write_le_bytes!(&self.hi);

        // Badvaddr and Cause registers are regnums 35 and 36
        write_le_bytes!(&self.cp0.badvaddr);
        write_le_bytes!(&self.cp0.cause);

        // Program Counter is regnum 37
        write_le_bytes!(&self.pc);

        // Write FPRs
        for reg in self.fpu.r.iter() {
            write_le_bytes!(&reg);
        }

        // Write FCSR and FIR registers
        write_le_bytes!(&self.fpu.fcsr);
        write_le_bytes!(&self.fpu.fir);
    }

    fn gdb_deserialize(&mut self, bytes: &[u8]) -> Result<(), ()> {
        let ptrsize = core::mem::size_of::<U>();

        // ensure bytes.chunks_exact(ptrsize) won't panic
        if bytes.len() % ptrsize != 0 {
            return Err(());
        }

        let mut regs = bytes
            .chunks_exact(ptrsize)
            .map(|c| U::from_le_bytes(c).unwrap());

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
