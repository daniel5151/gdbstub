//! Provide Wasm-specific actions for the target.
use crate::common::Tid;
use crate::target::Target;

/// Target Extension - perform Wasm-specific actions.
pub trait Wasm: Target {
    /// Get the Wasm call stack for a given thread.
    ///
    /// See the [LLDB source code] (particularly `WasmAddressType` and
    /// `wasm_addr_t`) for a description of the encoding of the PC
    /// values. These values are always 64 bits wide, even for 32-bit
    /// Wasm, as they encode the particular Wasm module plus an offset
    /// into that module's code space.
    ///
    /// [LLDB souce code]: https://github.com/llvm/llvm-project/blob/main/lldb/source/Plugins/Process/wasm/ProcessWasm.h
    ///
    /// To avoid allocation, the call stack PCs should be returned to
    /// the caller by calling the given callback, in order from
    /// innermost (most recently called) frame to outermost.
    fn wasm_call_stack(&self, tid: Tid, next_pc: &mut dyn FnMut(u64)) -> Result<(), Self::Error>;

    /// Get the Wasm local for a given thread, frame index, and local
    /// index.
    ///
    /// The Wasm local's value should be placed into `buf`, and the
    /// length should be returned. If the Wasm local or frame does not
    /// exist, this method should return `0`.
    ///
    /// `buf` is exactly 16 bytes long to allow for the largest
    /// possible Wasm value (a `v128` SIMD value). Values should be
    /// encoded in little-endian format with their native length
    /// (e.g., 4 bytes for a Wasm `i32` or `f32` type, or 8 bytes for
    /// a Wasm `i64` or `f64` type).
    fn read_wasm_local(
        &self,
        tid: Tid,
        frame: usize,
        local: usize,
        buf: &mut [u8; 16],
    ) -> Result<usize, Self::Error>;

    /// Get the Wasm operand-stack value for a given thread, frame
    /// index, and stack index. Top-of-stack is index 0, and values
    /// below that have incrementing indices.
    ///
    /// The Wasm operand's value should be placed into `buf`, and the
    /// length should be returned. If the Wasm local or frame does not
    /// exist, this method should return `0`.
    ///
    /// `buf` is exactly 16 bytes long to allow for the largest
    /// possible Wasm value (a `v128` SIMD value). Values should be
    /// encoded in little-endian format with their native length
    /// (e.g., 4 bytes for a Wasm `i32` or `f32` type, or 8 bytes for
    /// a Wasm `i64` or `f64` type).
    fn read_wasm_stack(
        &self,
        tid: Tid,
        frame: usize,
        index: usize,
        buf: &mut [u8; 16],
    ) -> Result<usize, Self::Error>;

    /// Get the Wasm global value for a given thread, frame, and
    /// global index. The global index is relative to the module whose
    /// function corresponds to that frame.
    ///
    /// The Wasm global's value should be placed into `buf`, and the
    /// length should be returned. If the Wasm local or frame does not
    /// exist, this method should return `0`.
    ///
    /// `buf` is exactly 16 bytes long to allow for the largest
    /// possible Wasm value (a `v128` SIMD value). Values should be
    /// encoded in little-endian format with their native length
    /// (e.g., 4 bytes for a Wasm `i32` or `f32` type, or 8 bytes for
    /// a Wasm `i64` or `f64` type).
    fn read_wasm_global(
        &self,
        tid: Tid,
        frame: usize,
        global: usize,
        buf: &mut [u8; 16],
    ) -> Result<usize, Self::Error>;
}

define_ext!(WasmOps, Wasm);
