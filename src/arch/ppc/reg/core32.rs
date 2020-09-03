use crate::arch::ppc::reg::PpcVector;
use crate::arch::Registers;

use core::convert::TryInto;

/// 32-bit PowerPC core registers.
///
/// Sources:
/// * https://github.com/bminor/binutils-gdb/blob/master/gdb/features/rs6000/powerpc-32.xml
/// * https://github.com/bminor/binutils-gdb/blob/master/gdb/features/rs6000/power-core.xml
/// * https://github.com/bminor/binutils-gdb/blob/master/gdb/features/rs6000/power-fpu.xml
#[derive(Default, Debug, PartialEq)]
pub struct PowerPcCoreRegs {
    /// General purpose registers
    pub r: [u32; 32],
    /// Float registers
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

impl Registers for PowerPcCoreRegs {
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
                    write_bytes!(&self.$reg.to_le_bytes());
                )*
            }
        }

        for reg in &self.r {
            write_bytes!(&reg.to_le_bytes());
        }

        for reg in &self.f {
            write_bytes!(&reg.to_le_bytes());
        }

        write_regs!(pc, msr, cr, lr, ctr, xer, fpscr);

        for &reg in &self.vr {
            let reg: u128 = reg.into();
            write_bytes!(&reg.to_le_bytes());
        }

        write_regs!(vscr, vrsave);
    }

    fn gdb_deserialize(&mut self, bytes: &[u8]) -> Result<(), ()> {
        if bytes.len() < 0x3a4 {
            return Err(());
        }

        let regs = bytes[0..0x80]
            .chunks_exact(4)
            .map(|x| u32::from_le_bytes(x.try_into().unwrap()));

        for (i, reg) in regs.enumerate() {
            self.r[i] = reg;
        }

        let regs = bytes[0x80..0x180]
            .chunks_exact(8)
            .map(|x| f64::from_le_bytes(x.try_into().unwrap()));

        for (i, reg) in regs.enumerate() {
            self.f[i] = reg;
        }

        macro_rules! parse_regs {
            ($start:literal..$end:literal, $($reg:ident),*) => {
                let mut regs = bytes[$start..$end]
                    .chunks_exact(4)
                    .map(|x| u32::from_le_bytes(x.try_into().unwrap()));
                $(
                    self.$reg = regs.next().ok_or(())?;
                )*
            }
        }

        parse_regs!(0x180..0x19c, pc, msr, cr, lr, ctr, xer, fpscr);

        let regs = bytes[0x19c..0x39c]
            .chunks_exact(0x10)
            .map(|x| u128::from_le_bytes(x.try_into().unwrap()));

        for (i, reg) in regs.enumerate() {
            self.vr[i] = reg.into();
        }

        parse_regs!(0x39c..0x3a4, vscr, vrsave);

        Ok(())
    }
}

#[cfg(test)]
mod ppc_core_tests {
    use super::*;

    #[test]
    fn ppc_core_round_trip() {
        let regs_before = PowerPcCoreRegs {
            r: [1; 32],
            pc: 2,
            msr: 3,
            cr: 4,
            lr: 5,
            ctr: 6,
            xer: 7,
            fpscr: 8,
            f: [9.0; 32],
            vr: [PpcVector::from([0u16, 1, 2, 3, 4, 5, 6, 7]); 32],
            vrsave: 10,
            vscr: 11,
        };

        let mut data = vec![];

        regs_before.gdb_serialize(|x| {
            data.push(x.unwrap_or(b'x'));
        });

        assert_eq!(data.len(), 0x3a4);

        let mut regs_after = PowerPcCoreRegs::default();
        regs_after.gdb_deserialize(&data).unwrap();

        assert_eq!(regs_before, regs_after);
    }
}
