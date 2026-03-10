//! Synthetic 64-bit Wasm address space expected by the LLDB Wasm extensions.
//!
//! WebAssembly is natively *multi-memory* and *multi-address-space*:
//!
//! - It supports zero or more "linear memories", and they have no canonical
//!   mapping into a single global address space for pointers; rather, each load
//!   and store instruction names which memory it accesses statically.
//! - It supports one or more "modules" containing first-class functions, and
//!   they have no canonical mapping into a single global code space; rather,
//!   control flow is structured, and calls between functions in different
//!   modules only occur via explicit strongly-typed function imports and
//!   exports.
//!
//! Wasm implementations typically represent these concepts directly rather than
//! attempt to map to a more conventional ISA model of a single flat address
//! space with machine code and data. However, the GDB RSP assumes the latter:
//! all of its commands, such as memory reads/writes, breakpoint updates, and
//! the like, use integers as pointers in a single address space.
//!
//! The LLDB Wasm extensions to the GDB RSP thus define a canonical mapping
//! between the multi-address-space world and a flat 64-bit address space for
//! the purposes of the protocol only. Note that this is 64-bit even when Wasm
//! natively has 32-bit memory offsets (the "wasm32" architecture), because the
//! definition adds additional information above the 32-bit offset.
//!
//! The [ProcessWasm.h] header file in the LLDB source contains definitions that
//! are as close to documentation as we can find: see the `WasmAddressType` and
//! `wasm_addr_t` definitions.
//!
//! An address consists of three parts:
//!
//! - The type: code or data. Wasm has separate "address spaces" for these, so
//!   they are mapped to different regions of the 64-bit synthetic space.\*
//!
//! - The module/memory index. The engine decides an arbitrary index ordering
//!   for all of the Wasm modules and Wasm linear memories present in a given
//!   execution.
//!
//! - The offset within that Wasm module bytecode or linear memory.
//!
//! \*Note that this implies that the original bytecode (the full image of the
//! Wasm module, starting with its magic number) is present in this synthetic
//! address space. An engine that implements debugging for Wasm should keep
//! around the original bytecode, even if it does ahead-of-time compilation or
//! other processing, so that the debugger can use it: LLDB will read the
//! module bytecode from the synthetic address space, including its debug
//! sections, rather than find the image elsewhere.
//!
//! [ProcessWasm.h]:
//!     https://github.com/llvm/llvm-project/blob/074653a/lldb/source/Plugins/Process/wasm/ProcessWasm.h

/// The type of an address in the synthetic address space used by the
/// Wasm target.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum WasmAddrType {
    /// Address in a 32-bit linear memory.
    Memory,
    /// Address in a `.wasm` module image.
    ///
    /// Used both for memory-read commands to fetch the Wasm binary
    /// from the gdbstub host, and software-breakpoint commands.
    Object,
}

/// An address in the synthetic address space used by the Wasm target.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WasmAddr(u64);

impl WasmAddr {
    const TYPE_BITS: u32 = 2;
    const MODULE_BITS: u32 = 30;
    const OFFSET_BITS: u32 = 32;

    const MODULE_SHIFT: u32 = Self::OFFSET_BITS;
    const TYPE_SHIFT: u32 = Self::OFFSET_BITS + Self::MODULE_BITS;

    const TYPE_MASK: u64 = (1u64 << Self::TYPE_BITS) - 1;
    const MODULE_MASK: u64 = (1u64 << Self::MODULE_BITS) - 1;
    const OFFSET_MASK: u64 = (1u64 << Self::OFFSET_BITS) - 1;

    /// Construct a `WasmAddr` from a raw 64-bit encoded address.
    ///
    /// Returns `None` if the encoding is invalid.
    pub fn from_raw(raw: u64) -> Option<Self> {
        let type_bits = (raw >> Self::TYPE_SHIFT) & Self::TYPE_MASK;
        if type_bits > 1 {
            return None;
        }
        Some(WasmAddr(raw))
    }

    /// Provide the raw 64-bit encoding of this `WasmAddr`.
    pub fn as_raw(self) -> u64 {
        self.0
    }

    /// Construct a `WasmAddr` from its constituent parts.
    pub fn new(addr_type: WasmAddrType, module_index: u32, offset: u32) -> Self {
        // There are fewer than 32 bits in the encoding for the module
        // index.
        assert_eq!(
            module_index >> Self::MODULE_BITS,
            0,
            "Out-of-bounds module index"
        );
        let type_bits: u64 = match addr_type {
            WasmAddrType::Memory => 0,
            WasmAddrType::Object => 1,
        };
        WasmAddr(
            (type_bits << Self::TYPE_SHIFT)
                | ((u64::from(module_index)) << Self::MODULE_SHIFT)
                | (u64::from(offset)),
        )
    }

    /// Get the type of this address.
    pub fn addr_type(self) -> WasmAddrType {
        match (self.0 >> Self::TYPE_SHIFT) & Self::TYPE_MASK {
            0 => WasmAddrType::Memory,
            1 => WasmAddrType::Object,
            // We never set other type-bits and the raw bits are fully
            // encapsulated and checked in `from_raw`, so this is
            // unreachable. `gdbstub_arch` forbids panics, so we
            // return a bogus type here.
            _ => WasmAddrType::Memory,
        }
    }

    /// Get the index of the module or memory referenced by this
    /// address.
    pub fn module_index(self) -> u32 {
        ((self.0 >> Self::MODULE_SHIFT) & Self::MODULE_MASK) as u32
    }

    /// Get the offset within the module or memory referenced by this
    /// address.
    pub fn offset(self) -> u32 {
        (self.0 & Self::OFFSET_MASK) as u32
    }
}

impl core::fmt::Display for WasmAddr {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let type_str = match self.addr_type() {
            WasmAddrType::Memory => "Memory",
            WasmAddrType::Object => "Object",
        };
        write!(
            f,
            "{}(module={}, offset={:#x})",
            type_str,
            self.module_index(),
            self.offset()
        )
    }
}

impl core::fmt::Debug for WasmAddr {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "WasmAddr({self})")
    }
}
