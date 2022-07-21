use core::convert::TryInto;

use gdbstub::arch::Registers;

/// AArch64 core registers.
///
/// Registers from the `org.gnu.gdb.aarch64.core` and `org.gnu.gdb.aarch64.fpu`
/// [AArch64 Standard GDB Target Features](https://sourceware.org/gdb/onlinedocs/gdb/AArch64-Features.html).
#[derive(Debug, Default, Clone, Eq, PartialEq)]
pub struct AArch64CoreRegs {
    /// General Purpose Registers (X0-X30)
    pub x: [u64; 31],
    /// Stack Pointer
    pub sp: u64,
    /// Program Counter
    pub pc: u64,
    /// Process State (GDB uses the AArch32 CPSR name)
    pub cpsr: u32,
}

impl Registers for AArch64CoreRegs {
    type ProgramCounter = u64;

    fn pc(&self) -> Self::ProgramCounter {
        self.pc
    }

    fn gdb_serialize(&self, mut write_byte: impl FnMut(Option<u8>)) {
        macro_rules! write_bytes {
            ($var: expr) => {
                for b in $var.to_le_bytes() {
                    write_byte(Some(b))
                }
            };
        }

        for reg in self.x.iter() {
            write_bytes!(reg);
        }
        write_bytes!(self.sp);
        write_bytes!(self.pc);
        write_bytes!(self.cpsr);
    }

    fn gdb_deserialize(&mut self, bytes: &[u8]) -> Result<(), ()> {
        const U64_END: usize = core::mem::size_of::<u64>() * 33;

        if bytes.len() % core::mem::size_of::<u32>() != 0 {
            return Err(());
        }

        let mut regs = bytes[0..U64_END]
            .chunks_exact(core::mem::size_of::<u64>())
            .map(|c| u64::from_le_bytes(c.try_into().unwrap()));

        for reg in self.x.iter_mut() {
            *reg = regs.next().ok_or(())?
        }
        self.sp = regs.next().ok_or(())?;
        self.pc = regs.next().ok_or(())?;

        let mut regs = bytes[U64_END..]
            .chunks_exact(core::mem::size_of::<u32>())
            .map(|c| u32::from_le_bytes(c.try_into().unwrap()));

        self.cpsr = regs.next().ok_or(())?;

        Ok(())
    }
}
