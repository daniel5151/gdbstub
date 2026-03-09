//! (LLDB extension) Provide Wasm-specific actions for the target.
//!
//! ### Address Encoding
//!
//! The gdbstub extension to the Wasm target architecture uses a
//! specific encoding for addresses, both for commands in this
//! extension trait and for commands in the base protocol (e.g., for
//! reading and writing memory and setting breakpoints). The need for
//! this scheme arises from the fact that Wasm is natively
//! "multimemory": there can be many code modules, and many linear
//! memories, and each is a native entity (rather than mapped into a
//! larger single address space) in the VM definition. The gdbstub
//! protocol extensions map these native entities into an address
//! space where the upper 32 bits encode the index of a particular
//! Wasm code module or linear (data) memory and the lower 32 bits
//! encode an offset.
//!
//! See the [LLDB source code] (particularly `WasmAddressType` and
//! `wasm_addr_t`) for a description of the encoding of the PC values.
//!
//! [LLDB souce code]: https://github.com/llvm/llvm-project/blob/main/lldb/source/Plugins/Process/wasm/ProcessWasm.h
use crate::common::Tid;
use crate::target::Target;

///  (LLDB extension) Target Extension - perform Wasm-specific actions.
pub trait Wasm: Target {
    /// Get the Wasm call stack for a given thread.
    ///
    /// The addresses provided for the PC at each frame shouuld be
    /// encoded as per the [Wasm address encoding].
    ///
    /// To avoid allocation, the call stack PCs should be returned to
    /// the caller by calling the given callback, in order from
    /// innermost (most recently called) frame to outermost.
    ///
    /// [Wasm address encoding]: `self#Address_Encoding`
    fn wasm_call_stack(&self, tid: Tid, next_pc: &mut dyn FnMut(u64)) -> Result<(), Self::Error>;

    /// Get the Wasm local for a given thread, frame index, and local
    /// index.
    ///
    /// The Wasm local's value should be placed into `buf`, and the
    /// length should be returned. If the Wasm local or frame does not
    /// exist, this method should return `0`.
    ///
    /// `buf` will be long enough to allow for the larget possible
    /// supported Wasm value (i.e., at least a `v128` SIMD
    /// value). Values should be encoded in little-endian format with
    /// their native length (e.g., 4 bytes for a Wasm `i32` or `f32`
    /// type, or 8 bytes for a Wasm `i64` or `f64` type).
    fn read_wasm_local(
        &self,
        tid: Tid,
        frame: usize,
        local: usize,
        buf: &mut [u8],
    ) -> Result<usize, Self::Error>;

    /// Get the Wasm operand-stack value for a given thread, frame
    /// index, and stack index. Top-of-stack is index 0, and values
    /// below that have incrementing indices.
    ///
    /// The Wasm operand's value should be placed into `buf`, and the
    /// length should be returned. If the Wasm local or frame does not
    /// exist, this method should return `0`.
    ///
    /// `buf` will be long enough to allow for the larget possible
    /// supported Wasm value (i.e., at least a `v128` SIMD
    /// value). Values should be encoded in little-endian format with
    /// their native length (e.g., 4 bytes for a Wasm `i32` or `f32`
    /// type, or 8 bytes for a Wasm `i64` or `f64` type).
    fn read_wasm_stack(
        &self,
        tid: Tid,
        frame: usize,
        index: usize,
        buf: &mut [u8],
    ) -> Result<usize, Self::Error>;

    /// Get the Wasm global value for a given thread, frame, and
    /// global index. The global index is relative to the module whose
    /// function corresponds to that frame.
    ///
    /// The Wasm global's value should be placed into `buf`, and the
    /// length should be returned. If the Wasm local or frame does not
    /// exist, this method should return `0`.
    ///
    /// `buf` will be long enough to allow for the larget possible
    /// supported Wasm value (i.e., at least a `v128` SIMD
    /// value). Values should be encoded in little-endian format with
    /// their native length (e.g., 4 bytes for a Wasm `i32` or `f32`
    /// type, or 8 bytes for a Wasm `i64` or `f64` type).
    fn read_wasm_global(
        &self,
        tid: Tid,
        frame: usize,
        global: usize,
        buf: &mut [u8],
    ) -> Result<usize, Self::Error>;
}

define_ext!(WasmOps, Wasm);
