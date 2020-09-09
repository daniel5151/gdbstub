use crate::arch::{RegId, Registers};

/// 32-bit ARM core register identifier.
#[derive(Debug)]
pub enum ArmCoreRegId {
    /// General purpose registers (R0-R12)
    Gpr(u8),
    /// Stack Pointer (R13)
    Sp,
    /// Link Register (R14)
    Lr,
    /// Program Counter (R15)
    Pc,
    /// Floating point registers (F0-F7)
    Fpr(u8),
    /// Floating point status
    Fps,
    /// Current Program Status Register (cpsr)
    Cpsr,
}

impl RegId for ArmCoreRegId {
    fn from_raw_id(id: usize) -> Option<(Self, usize)> {
        match id {
            0..=12 => Some((Self::Gpr(id as u8), 4)),
            13 => Some((Self::Sp, 4)),
            14 => Some((Self::Lr, 4)),
            15 => Some((Self::Pc, 4)),
            16..=23 => Some((Self::Fpr(id as u8), 4)),
            25 => Some((Self::Cpsr, 4)),
            _ => None,
        }
    }
}

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
    type RegId = ArmCoreRegId;

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

    fn gdb_deserialize(&mut self, bytes: &[u8]) -> Result<(), ()> {
        // ensure bytes.chunks_exact(4) won't panic
        if bytes.len() % 4 != 0 {
            return Err(());
        }

        use core::convert::TryInto;
        let mut regs = bytes
            .chunks_exact(4)
            .map(|c| u32::from_le_bytes(c.try_into().unwrap()));

        for reg in self.r.iter_mut() {
            *reg = regs.next().ok_or(())?
        }
        self.sp = regs.next().ok_or(())?;
        self.lr = regs.next().ok_or(())?;
        self.pc = regs.next().ok_or(())?;

        // Floating point registers (unused)
        for _ in 0..25 {
            regs.next().ok_or(())?;
        }

        self.cpsr = regs.next().ok_or(())?;

        if regs.next().is_some() {
            return Err(());
        }

        Ok(())
    }
}
