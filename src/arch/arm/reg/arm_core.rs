use crate::Registers;

/// 32-bit ARM core registers.
///
/// Source: https://github.com/bminor/binutils-gdb/blob/master/gdb/features/arm/arm-core.xml
#[derive(Default)]
pub struct ArmCoreRegs {
    /// General purpose registers (R0-R12)
    pub r: [u32; 13],
    /// Stack Pointer (R13)
    pub sp: u32,
    /// Link Register (R14)
    pub lr: u32,
    /// Program Counter (R15)
    pub pc: u32,
    /// Current Program Status Register (cpsr)
    pub cpsr: u32,
}

impl Registers for ArmCoreRegs {
    fn gdb_serialize(&self, mut write_byte: impl FnMut(Option<u8>)) {
        macro_rules! write_bytes {
            ($bytes:expr) => {
                for b in $bytes {
                    write_byte(Some(*b))
                }
            };
        }

        for reg in self.r.iter() {
            write_bytes!(&reg.to_le_bytes());
        }
        write_bytes!(&self.sp.to_le_bytes());
        write_bytes!(&self.lr.to_le_bytes());
        write_bytes!(&self.pc.to_le_bytes());

        // Floating point registers (unused)
        for _ in 0..25 {
            (0..4).for_each(|_| write_byte(None))
        }

        write_bytes!(&self.cpsr.to_le_bytes());
    }

    fn gdb_deserialize(&mut self, mut bytes: impl Iterator<Item = u8>) -> Result<(), ()> {
        let mut next_u32 = move || -> Option<u32> {
            let val = (bytes.next()? as u32)
                | (bytes.next()? as u32) << 8
                | (bytes.next()? as u32) << 16
                | (bytes.next()? as u32) << 24;
            Some(val)
        };

        for reg in self.r.iter_mut() {
            *reg = next_u32().ok_or(())?
        }
        self.sp = next_u32().ok_or(())?;
        self.lr = next_u32().ok_or(())?;
        self.pc = next_u32().ok_or(())?;

        // Floating point registers (unused)
        for _ in 0..25 {
            next_u32().ok_or(())?;
        }

        self.cpsr = next_u32().ok_or(())?;

        Ok(())
    }
}
