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
use core::num::NonZeroUsize;

use num_traits::{FromPrimitive, PrimInt, Unsigned};

use crate::internal::{BeBytes, LeBytes};

/// Register identifier for target registers.
///
/// These identifiers are used by GDB to signal which register to read/wite when
/// performing [single register accesses].
///
/// [single register accesses]:
/// crate::target::ext::base::single_register_access::SingleRegisterAccess
pub trait RegId: Sized + Debug {
    /// Map raw GDB register number to a corresponding `RegId` and optional
    /// register size.
    ///
    /// If the register size is specified here, gdbstub will include a runtime
    /// check that ensures target implementations do not send back more
    /// bytes than the register allows.
    ///
    /// Returns `None` if the register is not available.
    fn from_raw_id(id: usize) -> Option<(Self, Option<NonZeroUsize>)>;
}

/// Stub implementation -- Returns `None` for all raw IDs.
impl RegId for () {
    fn from_raw_id(_id: usize) -> Option<(Self, Option<NonZeroUsize>)> {
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
    type Usize: Debug + FromPrimitive + PrimInt + Unsigned + BeBytes + LeBytes;

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
    #[inline(always)]
    fn target_description_xml() -> Option<&'static str> {
        None
    }

    /// (optional) (LLDB extension) Write register info for one of the arch's
    /// register.
    ///
    /// Implementing this method enables LLDB to dynamically query the
    /// target's register information one by one.
    ///
    /// Some targets don't have register context in the compiled version
    /// of the debugger. Help the debugger by dynamically supplying the register
    /// info from the target. The debugger will request the register info in
    /// a sequential manner till an error packet is received. In LLDB, the
    /// register info search has the following [order](https://github.com/llvm/llvm-project/blob/369ce54bb302f209239b8ebc77ad824add9df089/lldb/source/Plugins/Process/gdb-remote/ProcessGDBRemote.cpp#L397-L402):
    ///
    ///1.    Use the target definition python file if one is specified.
    ///2.    If the target definition doesn't have any of the info from the
    ///target.xml (registers) then proceed to read the `target.xml`.
    ///3.    Fall back on the `qRegisterInfo` packets.
    ///4.    Use hardcoded defaults if available.
    ///
    /// See the LLDB [gdb-remote docs](https://github.com/llvm-mirror/lldb/blob/d01083a850f577b85501a0902b52fd0930de72c7/docs/lldb-gdb-remote.txt#L396)
    /// for more details on the available information that a single register can
    /// be described by and [#99](https://github.com/daniel5151/gdbstub/issues/99) for more information on LLDB compatibility.
    #[inline(always)]
    fn register_info(reg_id: usize) -> Option<lldb::RegisterInfo<'static>> {
        let _ = reg_id;
        None
    }

    /// Encode how the mainline GDB client handles target support for
    /// single-step on this particular architecture.
    ///
    /// # Context
    ///
    /// According to the spec, supporting single step _should_ be quite
    /// straightforward:
    ///
    /// - The GDB client sends a `vCont?` packet to enumerate supported
    ///   resumption modes
    /// - If the target supports single-step, it responds with the `s;S`
    ///   capability as part of the response, omitting it if it is not
    ///   supported.
    /// - Later, when the user attempts to `stepi`, the GDB client sends a `s`
    ///   resumption reason if it is supported, falling back to setting a
    ///   temporary breakpoint + continue to "emulate" the single step.
    ///
    /// Unfortunately, the reality is that the mainline GDB client does _not_ do
    /// this on all architectures...
    ///
    /// - On certain architectures (e.g: x86), GDB will _unconditionally_ assume
    ///   single-step support, regardless whether or not the target reports
    ///   supports it.
    /// - On certain architectures (e.g: MIPS), GDB will _never_ use single-step
    ///   support, even in the target has explicitly reported support for it.
    ///
    /// This is a bug, and has been reported at
    /// <https://sourceware.org/bugzilla/show_bug.cgi?id=28440>.
    ///
    /// For a easy repro of this behavior, also see
    /// <https://github.com/daniel5151/gdb-optional-step-bug>.
    ///
    /// # Implications
    ///
    /// Unfortunately, even if these idiosyncratic behaviors get fixed in the
    /// mainline GDB client, it will be quite a while until the typical
    /// user's distro-provided GDB client includes this bugfix.
    ///
    /// As such, `gdbstub` has opted to include this method as a "guard rail" to
    /// preemptively detect cases of this idiosyncratic behavior, and throw a
    /// pre-init error that informs the user of the potential issues they may
    /// run into.
    ///
    /// # Writing a proper implementation
    ///
    /// To check whether or not a particular architecture exhibits this
    /// behavior, an implementation should temporarily override this method to
    /// return [`SingleStepGdbBehavior::Optional`], toggle target support for
    /// single-step on/off, and observe the behavior of the GDB client after
    /// invoking `stepi`.
    ///
    /// If single-stepping was **disabled**, yet the client nonetheless sent a
    /// `vCont` packet with a `s` resume action, then this architecture
    /// _does not_ support optional single stepping, and this method should
    /// return [`SingleStepGdbBehavior::Required`].
    ///
    /// If single-stepping was **disabled**, and the client attempted to set a
    /// temporary breakpoint (using the `z` packet), and then sent a `vCont`
    /// packet with a `c` resume action, then this architecture _does_
    /// support optional single stepping, and this method should return
    /// [`SingleStepGdbBehavior::Optional`].
    ///
    /// If single-stepping was **enabled**, yet the client did _not_ send a
    /// `vCont` packet with a `s` resume action, then this architecture
    /// _ignores_ single stepping entirely, and this method should return
    /// [`SingleStepGdbBehavior::Ignored`].
    fn single_step_gdb_behavior() -> SingleStepGdbBehavior;
}

/// Encodes how the mainline GDB client handles target support for single-step
/// on a particular architecture.
///
/// See [Arch::single_step_gdb_behavior] for details.
#[non_exhaustive]
#[derive(Debug, Clone, Copy)]
pub enum SingleStepGdbBehavior {
    /// GDB will use single-stepping if available, falling back to using
    /// a temporary breakpoint + continue if unsupported.
    ///
    /// e.g: ARM
    Optional,
    /// GDB will unconditionally send single-step packets, _requiring_ the
    /// target to handle these requests.
    ///
    /// e.g: x86/x64
    Required,
    /// GDB will never use single-stepping, regardless if it's supported by the
    /// stub. It will always use a temporary breakpoint + continue.
    ///
    /// e.g: MIPS
    Ignored,
    /// Unknown behavior - no one has tested this platform yet. If possible,
    /// please conduct a test + upstream your findings to `gdbstub_arch`.
    #[doc(hidden)]
    Unknown,
}

/// LLDB-specific types supporting [`Arch::register_info`] and
/// [`RegisterInfoOverride`](
/// crate::target::ext::register_info_override::RegisterInfoOverride) APIs.
pub mod lldb {
    /// The architecture's register information of a single register.
    pub enum RegisterInfo<'a> {
        /// The register info of a single register that should be written.
        Register(Register<'a>),
        /// The `qRegisterInfo` query shall be concluded.
        Done,
    }

    /// Describes the register info for a single register of
    /// the target.
    pub struct Register<'a> {
        /// The primary register name.
        pub name: &'a str,
        /// An alternate name for the register.
        pub alt_name: Option<&'a str>,
        /// Size in bits of a register.
        pub bitsize: usize,
        /// The offset within the 'g' and 'G' packet of the register data for
        /// this register.
        pub offset: usize,
        /// The encoding type of the register.
        pub encoding: Encoding,
        /// The preferred format for display of this register.
        pub format: Format,
        /// The register set name this register belongs to.
        pub set: &'a str,
        /// The GCC compiler registers number for this register.
        /// _Note:_ This denotes the same `KEY:VALUE;` pair as `ehframe:VALUE;`.
        /// See the LLDB [source](https://github.com/llvm/llvm-project/blob/b92436efcb7813fc481b30f2593a4907568d917a/lldb/source/Plugins/Process/gdb-remote/ProcessGDBRemote.cpp#L493).
        pub gcc: Option<usize>,
        /// The DWARF register number for this register that is used for this
        /// register in the debug information.
        pub dwarf: Option<usize>,
        /// Specify as a generic register.
        pub generic: Option<Generic>,
        /// Other concrete register values this register is contained in.
        pub container_regs: Option<&'a [usize]>,
        /// Specifies which register values should be invalidated when this
        /// register is modified.
        pub invalidate_regs: Option<&'a [usize]>,
    }

    /// Describes the encoding type of the register.
    #[non_exhaustive]
    pub enum Encoding {
        /// Unsigned integer
        Uint,
        /// Signed integer
        Sint,
        /// IEEE 754 float
        IEEE754,
        /// Vector register
        Vector,
    }

    /// Describes the preferred format for display of this register.
    #[non_exhaustive]
    pub enum Format {
        /// Binary format
        Binary,
        /// Decimal format
        Decimal,
        /// Hexadecimal format
        Hex,
        /// Floating point format
        Float,
        /// 8 bit signed int vector
        VectorSInt8,
        /// 8 bit unsigned int vector
        VectorUInt8,
        /// 16 bit signed int vector
        VectorSInt16,
        /// 16 bit unsigned int vector
        VectorUInt16,
        /// 32 bit signed int vector
        VectorSInt32,
        /// 32 bit unsigned int vector
        VectorUInt32,
        /// 32 bit floating point vector
        VectorFloat32,
        /// 128 bit unsigned int vector
        VectorUInt128,
    }

    /// Describes the generic types that most CPUs have.
    #[non_exhaustive]
    pub enum Generic {
        /// Program counter register
        Pc,
        /// Stack pointer register
        Sp,
        /// Frame pointer register
        Fp,
        /// Return address register
        Ra,
        /// CPU flags register
        Flags,
        /// Function argument 1
        Arg1,
        /// Function argument 2
        Arg2,
        /// Function argument 3
        Arg3,
        /// Function argument 4
        Arg4,
        /// Function argument 5
        Arg5,
        /// Function argument 6
        Arg6,
        /// Function argument 7
        Arg7,
        /// Function argument 8
        Arg8,
    }
}
