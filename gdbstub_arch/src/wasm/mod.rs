//! Implementation for the WebAssembly architecture.
//!
//! This implementation follows the [LLDB-specific Wasm extensions] to the GDB
//! RSP, which define a mapping from Wasm concepts to more classical ISA
//! concepts.
//!
//! WebAssembly is somewhat *unlike* most ISAs in many of its details: for
//! example, it uses an operand stack rather than classical registers, and has
//! explicit concepts of function locals, of globals, and of first-class
//! functions and a callstack, rather than a flat address space of bytes that
//! are used to build up machine code, a stack, and storage as in most other
//! ISAs.
//!
//! As such - you'll need to implement the [`Wasm`] extension trait in your
//! `Target` implementation in order to provide LLDB access to these Wasm-native
//! concepts.
//!
//! As a particularly important detail, note that the natively
//! multi-address-space Wasm world, where multiple code modules exist without a
//! native concept of a global PC space, and multiple linear memories exist with
//! every load/store qualified by the memory it accesses, is mapped into a
//! single synthesized 64-bit address space by definition of the protocol
//! extensions. See the [`self::addr`] submodule for utilities to encode and
//! decode these synthesized addresses.
//!
//! [`Wasm`]: gdbstub::target::ext::wasm::Wasm
//! [LLDB-specific Wasm extensions]:
//!     https://lldb.llvm.org/resources/lldbgdbremote.html#wasm-packets

use gdbstub::arch::Arch;

pub mod addr;
pub mod reg;

/// Implements `Arch` for the WebAssembly architecture.
pub enum Wasm {}

impl Arch for Wasm {
    /// Even though Wasm is nominally a 32-bit platform, the gdbstub
    /// protocol for Wasm uses a 64-bit address word to multiplex module
    /// bytecode regions and linear memory regions into a single address
    /// space.
    type Usize = u64;
    type Registers = reg::WasmRegisters;
    type RegId = reg::id::WasmRegId;
    type BreakpointKind = usize;
}
