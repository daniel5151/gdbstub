use core::fmt::Debug;
use core::ops::Range;

use crate::Arch;

/// A collection of methods and metadata used by
/// [`GdbStub`](struct.GdbStub.html) to debug a system.
///
/// This trait describes the architecture and capabilities of a target system,
/// and provides an interface for `GdbStub` to modify and control the system's
/// state.
///
/// There are several [provided methods](#provided-methods) that can optionally
/// be implemented to enable additional advanced GDB debugging functionality.
/// Aside from overriding the method itself, each optional method has an
/// associated `fn impl_XXX(&self) -> bool` method which must also be overridden
/// to return `true`.
///
/// ### What's with the `<Self::Arch as Arch>::` syntax?
///
/// Yeah, sorry about that!
///
/// If [rust-lang/rust#38078](https://github.com/rust-lang/rust/issues/38078)
/// every gets fixed, `<Self::Arch as Arch>::Usize` can be simplified to the
/// much more readable `Self::Arch::Usize`.
///
/// Until then, when implementing `Target`, I recommend using the concrete
/// type directly. (e.g: on a 32-bit platform, instead of writing `<Self::Arch
/// as Arch>::Usize`, just use `u32` directly)
pub trait Target {
    /// The target's architecture.
    type Arch: Arch;

    /// A target-specific fatal error.
    type Error;

    /// Perform a single "step" of the emulated system. A step should be a
    /// single CPU instruction or less.
    fn step(&mut self) -> Result<TargetState<<Self::Arch as Arch>::Usize>, Self::Error>;

    /// Read the target's registers.
    fn read_registers(
        &mut self,
        regs: &mut <Self::Arch as Arch>::Registers,
    ) -> Result<(), Self::Error>;

    /// Write the target's registers.
    fn write_registers(
        &mut self,
        regs: &<Self::Arch as Arch>::Registers,
    ) -> Result<(), Self::Error>;

    /// Read the target's current PC.
    fn read_pc(&mut self) -> Result<<Self::Arch as Arch>::Usize, Self::Error>;

    /// Read bytes from the specified address range.
    fn read_addrs(
        &mut self,
        addrs: Range<<Self::Arch as Arch>::Usize>,
        val: impl FnMut(u8),
    ) -> Result<(), Self::Error>;

    /// Write bytes to the specified address range.
    fn write_addrs(
        &mut self,
        get_addr_val: impl FnMut() -> Option<(<Self::Arch as Arch>::Usize, u8)>,
    ) -> Result<(), Self::Error>;

    /// (optional) Target provides an `update_hw_breakpoint()` implementation.
    fn impl_update_hw_breakpoint(&self) -> bool {
        false
    }

    /// (optional) Update the target's hardware break/watchpoints. Returns a
    /// boolean indicating if the operation succeeded.
    ///
    /// As a convenience, `gdbstub` has built-in support for _Software_
    /// breakpoints, though implementing support for _Hardware_ breakpoints
    /// can substantially improve performance (especially when working with
    /// **memory watchpoints**).
    fn update_hw_breakpoint(
        &mut self,
        addr: <Self::Arch as Arch>::Usize,
        op: HwBreakOp,
    ) -> Result<bool, Self::Error> {
        let _ = (addr, op);
        unimplemented!();
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
