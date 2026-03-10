use core::convert::TryInto;
use gdbstub::arch::Registers;

/// The register state for WebAssembly.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WasmRegisters {
    /// Program Counter. See [`crate::wasm::addr`] for the 64-bit
    /// synthetic address space in which this PC exists.
    pub pc: u64,
}

impl Registers for WasmRegisters {
    type ProgramCounter = u64;

    fn pc(&self) -> u64 {
        self.pc
    }

    fn gdb_serialize(&self, mut write_byte: impl FnMut(Option<u8>)) {
        for byte in self.pc.to_le_bytes() {
            write_byte(Some(byte));
        }
    }

    fn gdb_deserialize(&mut self, bytes: &[u8]) -> Result<(), ()> {
        if bytes.len() < 8 {
            return Err(());
        }
        self.pc = u64::from_le_bytes(bytes[..8].try_into().unwrap());
        Ok(())
    }
}
