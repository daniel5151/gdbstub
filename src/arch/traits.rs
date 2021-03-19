use core::fmt::Debug;

use num_traits::{PrimInt, Unsigned};

use crate::internal::{BeBytes, LeBytes};

/// Register identifier for target registers.
///
/// These identifiers are used by GDB for single register operations.
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
    /// Serialize `self` into a GDB register bytestream.
    ///
    /// Missing registers are serialized by passing `None` to write_byte.
    fn gdb_serialize(&self, write_byte: impl FnMut(Option<u8>));

    /// Deserialize a GDB register bytestream into `self`.
    #[allow(clippy::clippy::result_unit_err)]
    fn gdb_deserialize(&mut self, bytes: &[u8]) -> Result<(), ()>;
}

/// Encodes architecture-specific information, such as pointer size, register
/// layout, etc...
///
/// Types implementing `Arch` should be
/// [Zero-variant Enums](https://doc.rust-lang.org/reference/items/enumerations.html#zero-variant-enums),
/// as they are only ever used at the type level, and should never be
/// instantiated.
pub trait Arch {
    /// The architecture's pointer size (e.g: `u32` on a 32-bit system).
    type Usize: PrimInt + Unsigned + BeBytes + LeBytes;

    /// The architecture's register file.
    type Registers: Registers;

    /// Register identifier enum/struct.
    ///
    /// Used to access individual registers via `Target::read/write_register`.
    ///
    /// NOTE: The `RegId` type is not required to have a 1:1 correspondence with
    /// the `Registers` type, and may include register identifiers which are
    /// separate from the main `Registers` structure.
    type RegId: RegId;

    /// (optional) Return the target's description XML file (`target.xml`).
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
