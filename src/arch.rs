//! Traits to encode architecture-specific target information.
//!
//! # Community created `Arch` Implementations
//!
//! Before getting your hands dirty and implementing a new `Arch` from scratch,
//! make sure to check out [`gdbstub_arch`](https://docs.rs/gdbstub_arch), a
//! companion crate to `gdbstub` which aggregates community-created `Arch`
//! implementations for most common architectures!
//!
//! > _Note:_ Prior to `gdbstub 0.5`, `Arch` implementations were distributed as
//! a part of the main `gdbstub` crate (under the `gdbstub::arch` module). This
//! wasn't ideal, any `gdbstub::arch`-level breaking-changes forced the _entire_
//! `gdbstub` crate to release a new (potentially breaking!) version.
//!
//! > Having community-created `Arch` implementations distributed in a separate
//! crate helps minimize any unnecessary "version churn" in `gdbstub` core.

use core::fmt::Debug;

use num_traits::{FromPrimitive, PrimInt, Unsigned};

use crate::internal::{BeBytes, LeBytes};

/// Register identifier for target registers.
///
/// These identifiers are used by GDB to signal which register to read/wite when
/// performing [single register accesses].
///
/// [single register accesses]: crate::target::ext::base::SingleRegisterAccess
pub trait RegId: Sized + Debug {
    /// Map raw GDB register number corresponding `RegId` and register size.
    ///
    /// Returns `None` if the register is not available.
    fn from_raw_id(id: usize) -> Option<(Self, usize)>;
}

/// Stub implementation -- Returns `None` for all raw IDs.
impl RegId for () {
    fn from_raw_id(_id: usize) -> Option<(Self, usize)> {
        None
    }
}

/// Methods to read/write architecture-specific registers.
///
/// Registers must be de/serialized in the order specified by the architecture's
/// `<target>.xml` in the GDB source tree.
///
/// e.g: for ARM:
/// github.com/bminor/binutils-gdb/blob/master/gdb/features/arm/arm-core.xml
// TODO: add way to de/serialize arbitrary "missing"/"uncollected" registers.
pub trait Registers: Default + Debug + Clone + PartialEq {
    /// The type of the architecture's program counter / instruction pointer.
    /// Must match with the corresponding `Arch::Usize`.
    type ProgramCounter: Copy;

    /// Return the value of the program counter / instruction pointer.
    fn pc(&self) -> Self::ProgramCounter;

    /// Serialize `self` into a GDB register bytestream.
    ///
    /// Missing registers are serialized by passing `None` to write_byte.
    fn gdb_serialize(&self, write_byte: impl FnMut(Option<u8>));

    /// Deserialize a GDB register bytestream into `self`.
    #[allow(clippy::result_unit_err)]
    fn gdb_deserialize(&mut self, bytes: &[u8]) -> Result<(), ()>;
}

/// Breakpoint kind for specific architectures.
///
/// This trait corresponds to the _kind_ field of the "z" and "Z" breakpoint
/// packets, as documented [here](https://sourceware.org/gdb/onlinedocs/gdb/Packets.html#insert-breakpoint-or-watchpoint-packet).
///
/// A breakpoint "kind" is architecture-specific and typically indicates the
/// size of the breakpoint in bytes that should be inserted. As such, most
/// architectures will set `BreakpointKind = usize`.
///
/// Some architectures, such as ARM and MIPS, have additional meanings for
/// _kind_. See the [Architecture-Specific Protocol Details](https://sourceware.org/gdb/current/onlinedocs/gdb/Architecture_002dSpecific-Protocol-Details.html#Architecture_002dSpecific-Protocol-Details)
/// section of the GBD documentation for more details.
///
/// If no architecture-specific value is being used, _kind_ should be set to
/// '0', and the `BreakpointKind` associated type should be `()`.
pub trait BreakpointKind: Sized + Debug {
    /// Parse `Self` from a raw usize.
    fn from_usize(kind: usize) -> Option<Self>;
}

impl BreakpointKind for () {
    fn from_usize(kind: usize) -> Option<Self> {
        if kind != 0 {
            None
        } else {
            Some(())
        }
    }
}

impl BreakpointKind for usize {
    #[allow(clippy::wrong_self_convention)]
    fn from_usize(kind: usize) -> Option<Self> {
        Some(kind)
    }
}

/// Encodes architecture-specific information, such as pointer size, register
/// layout, etc...
///
/// Types implementing `Arch` should be
/// [Zero-variant Enums](https://doc.rust-lang.org/reference/items/enumerations.html#zero-variant-enums),
/// as `Arch` impls are only ever used at the type level, and should never be
/// explicitly instantiated.
pub trait Arch {
    /// The architecture's pointer size (e.g: `u32` on a 32-bit system).
    type Usize: FromPrimitive + PrimInt + Unsigned + BeBytes + LeBytes;

    /// The architecture's register file. See [`Registers`] for more details.
    type Registers: Registers<ProgramCounter = Self::Usize>;

    /// The architecture's breakpoint "kind", used to determine the "size"
    /// of breakpoint to set. See [`BreakpointKind`] for more details.
    type BreakpointKind: BreakpointKind;

    /// Register identifier enum/struct.
    ///
    /// Used to access individual registers via `Target::read/write_register`.
    ///
    /// > NOTE: An arch's `RegId` type is not strictly required to have a 1:1
    /// correspondence with the `Registers` type, and may include register
    /// identifiers which are separate from the main `Registers` structure.
    /// (e.g: the RISC-V Control and Status registers)
    type RegId: RegId;

    /// (optional) Return the arch's description XML file (`target.xml`).
    ///
    /// Implementing this method enables GDB to automatically detect the
    /// target's architecture, saving the hassle of having to run `set
    /// architecture <arch>` when starting a debugging session.
    ///
    /// These descriptions can be quite succinct. For example, the target
    /// description for an `armv4t` target can be as simple as:
    ///
    /// ```
    /// r#"<target version="1.0"><architecture>armv4t</architecture></target>"#;
    /// ```
    ///
    /// See the [GDB docs](https://sourceware.org/gdb/current/onlinedocs/gdb/Target-Description-Format.html)
    /// for details on the target description XML format.
    fn target_description_xml() -> Option<&'static str> {
        None
    }
}
