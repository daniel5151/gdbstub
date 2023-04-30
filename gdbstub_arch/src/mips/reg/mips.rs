use core::convert::TryInto;
use gdbstub::arch::Registers;
use gdbstub::internal::LeBytes;
use num_traits::PrimInt;

/// MIPS registers.
///
/// The register width is set to `u32` or `u64` based on the `<U>` type.
///
/// Source: <https://github.com/bminor/binutils-gdb/blob/master/gdb/features/mips-cpu.xml>
#[derive(Debug, Default, Clone, Eq, PartialEq)]
pub struct MipsCoreRegs<U> {
    /// General purpose registers (R0-R31)
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
/// Source: <https://github.com/bminor/binutils-gdb/blob/master/gdb/features/mips-cp0.xml>
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
/// Source: <https://github.com/bminor/binutils-gdb/blob/master/gdb/features/mips-fpu.xml>
#[derive(Debug, Default, Clone, Eq, PartialEq)]
pub struct MipsFpuRegs<U> {
    /// FP registers (F0-F31) starting at regnum 38
    pub r: [U; 32],
    /// Floating-point Control Status register
    pub fcsr: U,
    /// Floating-point Implementation Register
    pub fir: U,
}

/// MIPS DSP registers.
///
/// Source: <https://github.com/bminor/binutils-gdb/blob/master/gdb/features/mips-dsp.xml>
#[derive(Debug, Default, Clone, Eq, PartialEq)]
pub struct MipsDspRegs<U> {
    /// High 1 register (regnum 72)
    pub hi1: U,
    /// Low 1 register (regnum 73)
    pub lo1: U,
    /// High 2 register (regnum 74)
    pub hi2: U,
    /// Low 2 register (regnum 75)
    pub lo2: U,
    /// High 3 register (regnum 76)
    pub hi3: U,
    /// Low 3 register (regnum 77)
    pub lo3: U,
    /// DSP Control register (regnum 78)
    /// Note: This register will always be 32-bit regardless of the target
    /// <https://sourceware.org/gdb/current/onlinedocs/gdb/MIPS-Features.html#MIPS-Features>
    pub dspctl: u32,
    /// Restart register (regnum 79)
    pub restart: U,
}

/// MIPS core and DSP registers.
///
/// Source: <https://github.com/bminor/binutils-gdb/blob/master/gdb/features/mips-dsp-linux.xml>
#[derive(Debug, Default, Clone, Eq, PartialEq)]
pub struct MipsCoreRegsWithDsp<U> {
    /// Core registers
    pub core: MipsCoreRegs<U>,
    /// DSP registers
    pub dsp: MipsDspRegs<U>,
}

impl<U> Registers for MipsCoreRegs<U>
where
    U: PrimInt + LeBytes + Default + core::fmt::Debug,
{
    type ProgramCounter = U;

    fn pc(&self) -> Self::ProgramCounter {
        self.pc
    }

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
            write_le_bytes!(reg);
        }

        // Write FCSR and FIR registers
        write_le_bytes!(&self.fpu.fcsr);
        write_le_bytes!(&self.fpu.fir);
    }

    fn gdb_deserialize(&mut self, bytes: &[u8]) -> Result<(), ()> {
        let ptrsize = core::mem::size_of::<U>();

        // Ensure bytes contains enough data for all 72 registers
        if bytes.len() < ptrsize * 72 {
            return Err(());
        }

        // All core registers are the same size
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

        Ok(())
    }
}

impl<U> Registers for MipsCoreRegsWithDsp<U>
where
    U: PrimInt + LeBytes + Default + core::fmt::Debug,
{
    type ProgramCounter = U;

    fn pc(&self) -> Self::ProgramCounter {
        self.core.pc
    }

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

        // Serialize the core registers first
        self.core.gdb_serialize(&mut write_byte);

        // Write the DSP registers
        write_le_bytes!(&self.dsp.hi1);
        write_le_bytes!(&self.dsp.lo1);
        write_le_bytes!(&self.dsp.hi2);
        write_le_bytes!(&self.dsp.lo2);
        write_le_bytes!(&self.dsp.hi3);
        write_le_bytes!(&self.dsp.lo3);

        for b in &self.dsp.dspctl.to_le_bytes() {
            write_byte(Some(*b));
        }

        write_le_bytes!(&self.dsp.restart);
    }

    fn gdb_deserialize(&mut self, bytes: &[u8]) -> Result<(), ()> {
        // Deserialize the core registers first
        self.core.gdb_deserialize(bytes)?;

        // Ensure bytes contains enough data for all 79 registers of target-width
        // and the dspctl register which is always 4 bytes
        let ptrsize = core::mem::size_of::<U>();
        if bytes.len() < (ptrsize * 79) + 4 {
            return Err(());
        }

        // Calculate the offsets to the DSP registers based on the ptrsize
        let dspregs_start = ptrsize * 72;
        let dspctl_start = ptrsize * 78;

        // Read up until the dspctl register
        let mut regs = bytes[dspregs_start..dspctl_start]
            .chunks_exact(ptrsize)
            .map(|c| U::from_le_bytes(c).unwrap());

        self.dsp.hi1 = regs.next().ok_or(())?;
        self.dsp.lo1 = regs.next().ok_or(())?;
        self.dsp.hi2 = regs.next().ok_or(())?;
        self.dsp.lo2 = regs.next().ok_or(())?;
        self.dsp.hi3 = regs.next().ok_or(())?;
        self.dsp.lo3 = regs.next().ok_or(())?;

        // Dspctl will always be a u32
        self.dsp.dspctl =
            u32::from_le_bytes(bytes[dspctl_start..dspctl_start + 4].try_into().unwrap());

        // Only 4 or 8 bytes should remain to be read
        self.dsp.restart = U::from_le_bytes(
            bytes[dspctl_start + 4..]
                .chunks_exact(ptrsize)
                .next()
                .ok_or(())?,
        )
        .unwrap();

        Ok(())
    }
}
