use gdbstub::arch::Registers;
use gdbstub::internal::LeBytes;
use num_traits::PrimInt;

/// TI-MSP430 registers.
///
/// The register width is set based on the `<U>` type. For 16-bit MSP430 CPUs
/// this should be `u16` and for 20-bit MSP430 CPUs (CPUX) this should be `u32`.
#[derive(Debug, Default, Clone, Eq, PartialEq)]
pub struct Msp430Regs<U> {
    /// Program Counter (R0)
    pub pc: U,
    /// Stack Pointer (R1)
    pub sp: U,
    /// Status Register (R2)
    pub sr: U,
    /// General Purpose Registers (R4-R15)
    pub r: [U; 12],
}

impl<U> Registers for Msp430Regs<U>
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
                let mut buf = [0; 4];
                // infallible (register size a maximum of 32-bits)
                let len = $value.to_le_bytes(&mut buf).unwrap();
                let buf = &buf[..len];
                for b in buf {
                    write_byte(Some(*b));
                }
            };
        }

        write_le_bytes!(&self.pc);
        write_le_bytes!(&self.sp);
        write_le_bytes!(&self.sr);
        (0..core::mem::size_of::<U>()).for_each(|_| write_byte(None)); // Constant Generator (CG/R3)
        for reg in self.r.iter() {
            write_le_bytes!(reg);
        }
    }

    fn gdb_deserialize(&mut self, bytes: &[u8]) -> Result<(), ()> {
        let ptrsize = core::mem::size_of::<U>();

        // Ensure bytes contains enough data for all 16 registers
        if bytes.len() < ptrsize * 16 {
            return Err(());
        }

        let mut regs = bytes
            .chunks_exact(ptrsize)
            .map(|c| U::from_le_bytes(c).unwrap());

        self.pc = regs.next().ok_or(())?;
        self.sp = regs.next().ok_or(())?;
        self.sr = regs.next().ok_or(())?;

        // Constant Generator (CG/R3) should always be 0
        if regs.next().ok_or(())? != U::zero() {
            return Err(());
        }

        for reg in self.r.iter_mut() {
            *reg = regs.next().ok_or(())?
        }

        if regs.next().is_some() {
            return Err(());
        }

        Ok(())
    }
}
