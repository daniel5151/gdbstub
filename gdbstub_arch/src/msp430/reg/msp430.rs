use gdbstub::arch::Registers;

/// 16-bit TI-MSP430 registers.
#[derive(Debug, Default, Clone, Eq, PartialEq)]
pub struct Msp430Regs {
    /// Program Counter (R0)
    pub pc: u16,
    /// Stack Pointer (R1)
    pub sp: u16,
    /// Status Register (R2)
    pub sr: u16,
    /// General Purpose Registers (R4-R15)
    pub r: [u16; 11],
}

impl Registers for Msp430Regs {
    type ProgramCounter = u16;

    fn pc(&self) -> Self::ProgramCounter {
        self.pc
    }

    fn gdb_serialize(&self, mut write_byte: impl FnMut(Option<u8>)) {
        macro_rules! write_bytes {
            ($bytes:expr) => {
                for b in $bytes {
                    write_byte(Some(*b))
                }
            };
        }

        write_bytes!(&self.pc.to_le_bytes());
        write_bytes!(&self.sp.to_le_bytes());
        write_bytes!(&self.sr.to_le_bytes());
        (0..4).for_each(|_| write_byte(None)); // Constant Generator (CG/R3)
        for reg in self.r.iter() {
            write_bytes!(&reg.to_le_bytes());
        }
    }

    fn gdb_deserialize(&mut self, bytes: &[u8]) -> Result<(), ()> {
        // ensure bytes.chunks_exact(2) won't panic
        if bytes.len() % 2 != 0 {
            return Err(());
        }

        use core::convert::TryInto;
        let mut regs = bytes
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes(c.try_into().unwrap()));

        self.pc = regs.next().ok_or(())?;
        self.sp = regs.next().ok_or(())?;
        self.sr = regs.next().ok_or(())?;

        // Constant Generator (CG/R3) should always be 0
        if regs.next().ok_or(())? != 0 {
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
