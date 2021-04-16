use gdbstub::arch::Registers;

use super::PpcVector;

use core::convert::TryInto;

/// 32-bit PowerPC core registers, FPU registers, and AltiVec SIMD registers.
///
/// Sources:
/// * https://github.com/bminor/binutils-gdb/blob/master/gdb/features/rs6000/powerpc-altivec32.xml
/// * https://github.com/bminor/binutils-gdb/blob/master/gdb/features/rs6000/power-core.xml
/// * https://github.com/bminor/binutils-gdb/blob/master/gdb/features/rs6000/power-fpu.xml
/// * https://github.com/bminor/binutils-gdb/blob/master/gdb/features/rs6000/power-altivec.xml
#[derive(Debug, Default, Clone, PartialEq)]
pub struct PowerPcCommonRegs {
    /// General purpose registers
    pub r: [u32; 32],
    /// Floating Point registers
    pub f: [f64; 32],
    /// Program counter
    pub pc: u32,
    /// Machine state
    pub msr: u32,
    /// Condition register
    pub cr: u32,
    /// Link register
    pub lr: u32,
    /// Count register
    pub ctr: u32,
    /// Integer exception register
    pub xer: u32,
    /// Floating-point status and control register
    pub fpscr: u32,
    /// Vector registers
    pub vr: [PpcVector; 32],
    /// Vector status and control register
    pub vscr: u32,
    /// Vector context save register
    pub vrsave: u32,
}

impl Registers for PowerPcCommonRegs {
    type ProgramCounter = u32;

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

        macro_rules! write_regs {
            ($($reg:ident),*) => {
                $(
                    write_bytes!(&self.$reg.to_be_bytes());
                )*
            }
        }

        for reg in &self.r {
            write_bytes!(&reg.to_be_bytes());
        }

        for reg in &self.f {
            write_bytes!(&reg.to_be_bytes());
        }

        write_regs!(pc, msr, cr, lr, ctr, xer, fpscr);

        for &reg in &self.vr {
            let reg: u128 = reg;
            write_bytes!(&reg.to_be_bytes());
        }

        write_regs!(vscr, vrsave);
    }

    fn gdb_deserialize(&mut self, bytes: &[u8]) -> Result<(), ()> {
        if bytes.len() < 0x3a4 {
            return Err(());
        }

        let mut regs = bytes[0..0x80]
            .chunks_exact(4)
            .map(|x| u32::from_be_bytes(x.try_into().unwrap()));

        for reg in &mut self.r {
            *reg = regs.next().ok_or(())?;
        }

        let mut regs = bytes[0x80..0x180]
            .chunks_exact(8)
            .map(|x| f64::from_be_bytes(x.try_into().unwrap()));

        for reg in &mut self.f {
            *reg = regs.next().ok_or(())?;
        }

        macro_rules! parse_regs {
            ($start:literal..$end:literal, $($reg:ident),*) => {
                let mut regs = bytes[$start..$end]
                    .chunks_exact(4)
                    .map(|x| u32::from_be_bytes(x.try_into().unwrap()));
                $(
                    self.$reg = regs.next().ok_or(())?;
                )*
            }
        }

        parse_regs!(0x180..0x19c, pc, msr, cr, lr, ctr, xer, fpscr);

        let mut regs = bytes[0x19c..0x39c]
            .chunks_exact(0x10)
            .map(|x| u128::from_be_bytes(x.try_into().unwrap()));

        for reg in &mut self.vr {
            *reg = regs.next().ok_or(())?;
        }

        parse_regs!(0x39c..0x3a4, vscr, vrsave);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ppc_core_round_trip() {
        let regs_before = PowerPcCommonRegs {
            r: [1; 32],
            pc: 2,
            msr: 3,
            cr: 4,
            lr: 5,
            ctr: 6,
            xer: 7,
            fpscr: 8,
            f: [9.0; 32],
            vr: [52; 32],
            vrsave: 10,
            vscr: 11,
        };

        let mut data = vec![];

        regs_before.gdb_serialize(|x| {
            data.push(x.unwrap_or(b'x'));
        });

        assert_eq!(data.len(), 0x3a4);

        let mut regs_after = PowerPcCommonRegs::default();
        regs_after.gdb_deserialize(&data).unwrap();

        assert_eq!(regs_before, regs_after);
    }
}
