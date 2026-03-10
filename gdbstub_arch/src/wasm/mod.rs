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
//! To use `gdbstub` with the LLDB Wasm GDB RSP extensions:
//!
//! 1. Implement the `Target` trait and the [`Wasm`], [`HostInfo`] and
//!    [`ProcessInfo`] traits on the target implementation for your Wasm
//!    execution engine/target.
//! 2. Make use of this `Arch` implementation in your target.
//! 3. Make use of the [`report_stop_with_regs`] API to report the Wasm PC with
//!    every stop packet.
//!    - _Note_: It seems likely that this requirement stems from a LLDB bug, as
//!      "expedited registers" are not typically mandated by the GDB RSP, and
//!      generally serve as an optional optimization to reduce roundtrips.
//! 4. Ensure that you have a build of LLDB with the Wasm target enabled. (A
//!    binary distribution of LLDB with your operating system may not have this,
//!    but a build from LLVM source will, by default. Once a release of
//!    [`wasi-sdk`] with [this PR] is made, `wasi-sdk` will distribute such a
//!    build for all major platforms.)
//! 5. Start up LLDB and attach it to an endpoint served by `gdbstub` with this
//!    target:
//!
//!    ```text
//!    $ .../bin/lldb
//!    (lldb) process connect --plugin wasm connect://localhost:1234
//!    ```
//!
//!    then ordinary debugging with breakpoints, step/continue, and state
//!    examination should work.
//!
//! See [Wasmtime] for an example of the use of this crate.
//!
//! [LLDB-specific Wasm extensions]:
//!     https://lldb.llvm.org/resources/lldbgdbremote.html#wasm-packets
//! [`Wasm`]: gdbstub::target::ext::wasm::Wasm
//! [`HostInfo`]: gdbstub::target::ext::host_info::HostInfo
//! [`ProcessInfo`]: gdbstub::target::ext::process_info::ProcessInfo
//! [`report_stop_with_regs`]:
//!     gdbstub::stub::state_machine::GdbStubStateMachineInner::report_stop_with_regs
//! [`wasi-sdk`]: https://github.com/WebAssembly/wasi-sdk
//! [this PR]: https://github.com/WebAssembly/wasi-sdk/pull/596
//! [Wasmtime]: https://github.com/bytecodealliance/wasmtime

use gdbstub::arch::Arch;

pub mod addr;
pub mod reg;

/// Implements `Arch` for the WebAssembly architecture.
pub enum Wasm {}

impl Arch for Wasm {
    /// Even though Wasm is nominally a 32-bit platform, LLDB's GDB RSP
    /// extensions for Wasm uses a 64-bit address word to multiplex module
    /// bytecode regions and linear memory regions into a single address space.
    type Usize = u64;
    type Registers = reg::WasmRegisters;
    type RegId = reg::id::WasmRegId;
    type BreakpointKind = usize;
}
