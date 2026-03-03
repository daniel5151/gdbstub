//! Provide Wasm-specific actions for the target.
use crate::{common::Tid, target::Target};

/// Target Extension - perform Wasm-specific actions.
pub trait Wasm: Target {
    /// Get the Wasm call stack for a given thread.
    ///
    /// See the [LLDB Wasm Extension Documentation] for a description
    /// of the format of the PC values.
    ///
    /// [LLDB Wasm Extension Documentation]: https://lldb.llvm.org/resources/lldbgdbremote.html#wasm-packets
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
    fn read_wasm_local(
        &self,
        tid: Tid,
        frame: u32,
        local: u32,
        buf: &mut [u8; 16],
    ) -> Result<usize, Self::Error>;

    /// Get the Wasm operand-stack value for a given thread, frame
    /// index, and stack index. Top-of-stack is index 0, and values
    /// below that have incrementing indices.
    ///
    /// The Wasm operand's value should be placed into `buf`, and the
    /// length should be returned. If the Wasm local or frame does not
    /// exist, this method should return `0`.
    fn read_wasm_stack(
        &self,
        tid: Tid,
        frame: u32,
        index: u32,
        buf: &mut [u8; 16],
    ) -> Result<usize, Self::Error>;

    /// Get the Wasm global value for a given thread, frame, and
    /// global index. The global index is relative to the module whose
    /// function corresponds to that frame.
    ///
    /// The Wasm global's value should be placed into `buf`, and the
    /// length should be returned. If the Wasm local or frame does not
    /// exist, this method should return `0`.
    fn read_wasm_global(
        &self,
        tid: Tid,
        frame: u32,
        global: u32,
        buf: &mut [u8; 16],
    ) -> Result<usize, Self::Error>;
}

define_ext!(WasmOps, Wasm);
