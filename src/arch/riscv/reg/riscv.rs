use crate::arch::{RegId, Registers};
use crate::internal::LeBytes;
use num_traits::PrimInt;

/// RISC-V Register identifier.
#[derive(Debug, Clone)]
pub enum RiscvRegId {
    /// General Purpose Register (x0-x31).
    Gpr(u8),
    /// Floating Point Register (f0-f31).
    Fpr(u8),
    /// Program Counter.
    Pc,
    /// Control and Status Register.
    Csr(u16),
    /// Privilege level.
    Priv,
}

impl RegId for RiscvRegId {
    fn from_raw_id(id: usize) -> Option<(Self, usize)> {
        match id {
            0..=31 => Some((Self::Gpr(id as u8), 4)),
            32 => Some((Self::Pc, 4)),
            33..=64 => Some((Self::Fpr((id - 33) as u8), 4)),
            65..=4160 => Some((Self::Csr((id - 65) as u16), 4)),
            4161 => Some((Self::Priv, 1)),
            _ => None,
        }
    }
}

/// RISC-V Integer registers.
///
/// The register width is set to `u32` or `u64` based on the `<U>` type.
///
/// Useful links:
/// * [GNU binutils-gdb XML descriptions](https://github.com/bminor/binutils-gdb/blob/master/gdb/features/riscv)
/// * [riscv-tdep.h](https://github.com/bminor/binutils-gdb/blob/master/gdb/riscv-tdep.h)
#[derive(Default)]
pub struct RiscvCoreRegs<U> {
    /// General purpose registers (x0-x31)
    pub x: [U; 32],
    /// Program counter
    pub pc: U,
}

impl<U> Registers for RiscvCoreRegs<U>
where
    U: PrimInt + LeBytes + Default,
{
    type RegId = RiscvRegId;

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
        for reg in self.x.iter() {
            write_le_bytes!(reg);
        }

        // Program Counter is regnum 33
        write_le_bytes!(&self.pc);
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
        for reg in self.x.iter_mut() {
            *reg = regs.next().ok_or(())?
        }
        self.pc = regs.next().ok_or(())?;

        if regs.next().is_some() {
            return Err(());
        }

        Ok(())
    }
}
