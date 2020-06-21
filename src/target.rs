use core::fmt::{self, Debug};
use core::ops::Range;

use num_traits::{Num, PrimInt, Unsigned};

/// Describes a target system which can be debugged using
/// [`GdbStub`](struct.GdbStub.html).
///
/// This trait describes the architecture and capabilities of a target system,
/// and provides an interface for `GdbStub` to modify and control the system's
/// state.
///
/// Several of the trait's "Provided methods" can be overwritten to enable
/// certain advanced GDB debugging features. For example, the
/// [`target_description_xml`](#method.target_description_xml) method can be
/// overwritten to enable automatic architecture detection.
///
/// ### What's `<target>.xml`?
///
/// Some required methods rely on target-specific information which can only be
/// found in GDB's internal `<target>.xml` files. For example, a basic 32-bit
/// ARM target uses the register layout described in the
///  [`arm-core.xml`](https://github.com/bminor/binutils-gdb/blob/master/gdb/features/arm/arm-core.xml)
/// file.
// TODO: Introduce a `Registers` trait to abstract register read/write
//  - i.e: provide "built-in" `Registers` implementations for common
//    architectures which match GDB's XML files.
//  - always easier to work with structured data instead of unstructured data...
pub trait Target {
    /// The target architecture's pointer size (e.g: `u32` on a 32-bit system).
    type Usize: Num + PrimInt + Unsigned + Debug + fmt::LowerHex;

    /// A target-specific fatal error.
    type Error;

    /// Perform a single "step" of the emulated system. A step should be a
    /// single CPU instruction or less.
    ///
    /// The provided `log_mem_access` function should be called each time a
    /// memory location is accessed.
    fn step(&mut self) -> Result<TargetState<Self::Usize>, Self::Error>;

    /// Read the target's registers.
    ///
    /// The registers should be read in the order specified in the
    /// [`<target>.xml`](#whats-targetxml). The provided `push_reg` function
    /// should be called with the register's value.
    // e.g: for ARM: binutils-gdb/blob/master/gdb/features/arm/arm-core.xml
    fn read_registers(&mut self, push_reg: impl FnMut(&[u8])) -> Result<(), Self::Error>;

    /// Write the target's registers.
    ///
    /// The bytes are provided in the order specified in the target's registers
    /// are provided in the order specified in the
    /// [`<target>.xml`](#whats-targetxml).
    ///
    /// e.g: for ARM: binutils-gdb/blob/master/gdb/features/arm/arm-core.xml
    fn write_registers(&mut self, pop_reg: impl FnMut() -> Option<u8>) -> Result<(), Self::Error>;

    /// Read the target's current PC.
    fn read_pc(&mut self) -> Result<Self::Usize, Self::Error>;

    /// Read bytes from the specified address range.
    fn read_addrs(
        &mut self,
        addrs: Range<Self::Usize>,
        val: impl FnMut(u8),
    ) -> Result<(), Self::Error>;

    /// Write bytes to the specified address range.
    fn write_addrs(
        &mut self,
        get_addr_val: impl FnMut() -> Option<(Self::Usize, u8)>,
    ) -> Result<(), Self::Error>;

    /// (optional) Return the platform's `features.xml` file.
    ///
    /// Implementing this method enables `gdb` to automatically detect the
    /// target's architecture, saving the hassle of having to run `set
    /// architecture <arch>` when starting a debugging session.
    ///
    /// These descriptions can be quite succinct. For example, the target
    /// description for an `armv4t` platform can be as simple as:
    ///
    /// ```
    /// r#"<target version="1.0"><architecture>armv4t</architecture></target>"#
    /// # ;
    /// ```
    ///
    /// See the [GDB docs](https://sourceware.org/gdb/current/onlinedocs/gdb/Target-Description-Format.html)
    /// for details on the target description XML format.
    fn target_description_xml() -> Option<&'static str> {
        None
    }

    /// (optional) Update the target's hardware break/watchpoints. Returns a
    /// boolean indicating if the operation succeeded.
    ///
    /// While `gdbstub` has built-in support for _Software_ breakpoints,
    /// implementing support for _Hardware_ breakpoints can substantially
    /// improve performance (especially when working with **memory
    /// watchpoints**).
    fn update_hw_breakpoint(
        &mut self,
        addr: Self::Usize,
        op: HwBreakOp,
    ) -> Option<Result<bool, Self::Error>> {
        let _ = (addr, op);

        None
    }
}

/// What kind of watchpoint.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WatchKind {
    /// Fire when the memory location is written to.
    Write,
    /// Fire when the memory location is read from.
    Read,
    /// Fire when the memory location is written to and/or read from.
    ReadWrite,
}

/// Add/Remove hardware breakpoints / watchpoints
#[derive(Debug)]
pub enum HwBreakOp {
    /// Add a new hardware breakpoint at specified address.
    AddBreak,
    /// Add a new watchpoint for the specified address.
    AddWatch(WatchKind),
    /// Remove the hardware breakpoint
    RemoveBreak,
    /// Remove the hardware watchpoint
    RemoveWatch(WatchKind),
}

/// The system's current execution state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TargetState<U> {
    /// Running
    Running,
    /// Halted
    Halted,
    /// Hit a software breakpoint (e.g. due to a trap instruction).
    ///
    /// NOTE: This does not necessarily have to be a breakpoint configured by
    /// the client/user of the current GDB session.
    SwBreak,
    /// Hit a hardware breakpoint.
    HwBreak,
    /// Hit a watchpoint.
    Watch {
        /// What kind of Watchpoint was hit
        kind: WatchKind,
        /// Associated data address
        addr: U,
    },
}
