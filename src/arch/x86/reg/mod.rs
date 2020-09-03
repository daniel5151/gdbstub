//! `GdbRegister` structs for x86 architectures.

use crate::arch::Registers;
use core::convert::TryInto;

mod core32;
mod core64;

pub use core32::X86CoreRegs;
pub use core64::X86_64CoreRegs;

/// 80-bit floating point value
pub type F80 = [u8; 10];

/// FPU registers
#[derive(Default)]
pub struct X87FpuInternalRegs {
    /// Floating-point control register
    pub fctrl: u32,
    /// Floating-point status register
    pub fstat: u32,
    /// Tag word
    pub ftag: u32,
    /// FPU instruction pointer segment
    pub fiseg: u32,
    /// FPU intstruction pointer offset
    pub fioff: u32,
    /// FPU operand segment
    pub foseg: u32,
    /// FPU operand offset
    pub fooff: u32,
    /// Floating-point opcode
    pub fop: u32,
}

impl Registers for X87FpuInternalRegs {
    type RegId = ();

    fn gdb_serialize(&self, mut write_byte: impl FnMut(Option<u8>)) {
        macro_rules! write_bytes {
            ($bytes:expr) => {
                for b in $bytes {
                    write_byte(Some(*b))
                }
            };
        }

        // Note: GDB section names don't make sense unless you read x87 FPU section 8.1:
        // https://web.archive.org/web/20150123212110/http://www.intel.com/content/dam/www/public/us/en/documents/manuals/64-ia-32-architectures-software-developer-vol-1-manual.pdf
        write_bytes!(&self.fctrl.to_le_bytes());
        write_bytes!(&self.fstat.to_le_bytes());
        write_bytes!(&self.ftag.to_le_bytes());
        write_bytes!(&self.fiseg.to_le_bytes());
        write_bytes!(&self.fioff.to_le_bytes());
        write_bytes!(&self.foseg.to_le_bytes());
        write_bytes!(&self.fooff.to_le_bytes());
        write_bytes!(&self.fop.to_le_bytes());
    }

    fn gdb_deserialize(&mut self, bytes: &[u8]) -> Result<(), ()> {
        if bytes.len() != 0x20 {
            return Err(());
        }

        let mut regs = bytes
            .chunks_exact(4)
            .map(|x| u32::from_le_bytes(x.try_into().unwrap()));

        self.fctrl = regs.next().ok_or(())?;
        self.fstat = regs.next().ok_or(())?;
        self.ftag = regs.next().ok_or(())?;
        self.fiseg = regs.next().ok_or(())?;
        self.fioff = regs.next().ok_or(())?;
        self.foseg = regs.next().ok_or(())?;
        self.fooff = regs.next().ok_or(())?;
        self.fop = regs.next().ok_or(())?;

        Ok(())
    }
}
