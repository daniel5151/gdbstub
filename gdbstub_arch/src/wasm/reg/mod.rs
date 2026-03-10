//! `Register` structs for the WebAssembly architecture.
//!
//! Because Wasm is mostly stack-based, it only has one "register":
//! the program counter (PC) according to the gdbstub mappings for
//! this architecture.

/// `RegId` definitions for WebAssembly.
pub mod id;

mod wasm_regs;

pub use wasm_regs::WasmRegisters;
