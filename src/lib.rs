//! An implementation of the
//! [GDB Remote Serial Protocol](https://sourceware.org/gdb/onlinedocs/gdb/Remote-Protocol.html#Remote-Protocol)
//! in Rust.
//!
//! `gdbstub` tries to make as few assumptions as possible about a project's
//! architecture, and aims to provide a "drop-in" way to add GDB support,
//! _without_ requiring any large refactoring / ownership juggling. It is
//! particularly useful in _emulators_, where it provides a powerful,
//! non-intrusive way to debug code running within an emulated system.
//!
//! **Disclaimer:** `gdbstub` is still in it's early stages of development!
//! Expect breaking API changes between minor releases.
//!
//! ## Debugging Features
//!
//! At the moment, `gdbstub` implements enough of the GDB Remote Serial Protocol
//! to support step-through + breakpoint debugging of single-threaded code.
//!
//! - Core GDB Protocol
//!     - Step + Continue
//!     - Add + Remove Breakpoints
//!     - Read/Write memory
//!     - Read/Write registers
//!     - Read/Write/Access Watchpoints (i.e: value breakpoints)
//! - Extended GDB Protocol
//!     - (optional) Automatic architecture detection
//!
//! The GDB Remote Serial Protocol is surprisingly complex, supporting advanced
//! features such as remote file I/O, spawning new processes, "rewinding"
//! program execution, and much, _much_ more. Thankfully, most of these features
//! are completely optional, and getting a basic debugging session
//! up-and-running only requires a small subset of commands to be implemented.
//!
//! ## Feature flags
//!
//! `gdbstub` is `no_std` by default, though it does have a dependency on
//! `alloc`.
//!
//! Additional functionality can be enabled by activating certain features.
//!
//! - `std` - (disabled by default)
//!   - Implements [`Connection`](trait.Connection.html) for
//!     `std::net::TcpStream`.
//!   - Implements [`std::error::Error`](https://doc.rust-lang.org/std/error/trait.Error.html)
//!     for [`gdbstub::Error`](enum.Error.html).
//!   - Outputs protocol responses via `log::trace!`
//!
//! ## Integrating `gdbstub` Into an Existing Project
//!
//! **Note:** This section provides a rough overview of what a `gdbstub`
//! integration might look like. It is _not_ a fully-fledged working example.
//! Please refer to examples included in the crate's `examples` folder + those
//! listed under [Real-World Examples](#real-world-examples) for examples that
//! can be compiled and run.
//!
//! Consider a project with the following structure:
//!
//! ```ignore
//! struct EmuError { /* ... */ }
//!
//! struct Emu { /* ... */ }
//! impl Emu {
//!     /// tick the system a single instruction
//!     fn step(&mut self) -> Result<(), EmuError> { /* ... */ }
//!     /// read a register's value
//!     fn read_reg(&self, idx: usize) -> u32 { /* ... */  }
//!     /// read a byte from a given address
//!     fn r8(&mut self, addr: u32) -> u8 { /* ... */ }
//!     // ... etc ...
//! }
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let mut emu = Emu::new();
//!     loop {
//!         emu.step()?;
//!     }
//! }
//! ```
//!
//! ### The `Target` trait
//!
//! The [`Target`](trait.Target.html) trait is used to modify and control a
//! system's execution state during a GDB debugging session. Since each project
//! is different, it's up to the user to provide methods to read/write memory,
//! step execution, etc...
//!
//! ```ignore
//! use gdbstub::{GdbStub, Target, TargetState};
//!
//! impl Target for Emu {
//!     // The target's pointer size.
//!     type Usize = u32;
//!     // Project-specific error type.
//!     type Error = EmuError;
//!
//!     // Run the system for a single "step" (i.e: one instruction or less)
//!     fn step(&mut self) -> Result<TargetState, Self::Error> {
//!         // run the system
//!         self.step()?; // <-- can use `?` to propagate project-specific errors!
//!         Ok(TargetState::Running)
//!     }
//!
//!     // Read-out the CPU's register values in the order specified in the arch's
//!     // `target.xml` file.
//!     // e.g: for ARM: binutils-gdb/blob/master/gdb/features/arm/arm-core.xml
//!     fn read_registers(&mut self, mut push_reg: impl FnMut(&[u8])) -> Result<(), Self::Error> {
//!         // general purpose registers
//!         for i in 0..13 {
//!             push_reg(&self.cpu.reg_get(i).to_le_bytes());
//!         }
//!         push_reg(&self.cpu.reg_get(reg::SP).to_le_bytes());
//!         push_reg(&self.cpu.reg_get(reg::LR).to_le_bytes());
//!         push_reg(&self.cpu.reg_get(reg::PC).to_le_bytes());
//!         // Floating point registers, unused
//!         for _ in 0..25 {
//!             push_reg(&[0, 0, 0, 0]);
//!         }
//!         push_reg(&self.cpu.reg_get(reg::CPSR).to_le_bytes());
//!
//!         Ok(())
//!     }
//!
//!     // Write to the CPU's register values in the order specified in the arch's
//!     // `target.xml` file.
//!     fn write_registers(&mut self, regs: &[u8]) -> Result<(), Self::Error> {
//!         /* ... similar to read_registers ... */
//!     }
//!
//!     fn read_pc(&mut self) -> u32 {
//!         self.cpu.reg_get(reg::PC)
//!     }
//!
//!     // read the specified memory addresses from the target
//!     fn read_addrs(
//!         &mut self,
//!         addr: std::ops::Range<u32>,
//!         mut push_byte: impl FnMut(u8)
//!     ) -> Result<(), Self::Error> {
//!         for addr in addr {
//!             push_byte(self.mem.r8(addr))
//!         }
//!         Ok(())
//!     }
//!
//!     // write data to the specified memory addresses
//!     fn write_addrs(
//!         &mut self,
//!         mut get_addr_val: impl FnMut() -> Option<(u32, u8)>
//!     ) -> Result<(), Self::Error> {
//!         while let Some((addr, val)) = get_addr_val() {
//!             self.mem.w8(addr, val);
//!         }
//!         Ok(())
//!     }
//!
//!     // there are several other methods whose default implementations can be
//!     // overridden to enable certain advanced GDB features
//!     // (e.g: automatic arch detection).
//!     //
//!     // See the docs for details.
//! }
//! ```
//!
//! ### The `Connection` trait
//!
//! The GDB Remote Serial Protocol is transport agnostic, only requiring that
//! the transport provides in-order, bytewise I/O (such as TCP, UDS, UART,
//! etc...). This transport requirement is encoded in the
//! [`Connection`](trait.Connection.html) trait.
//!
//! `gdbstub` includes a pre-defined implementation of `Connection` for
//! `std::net::TcpStream` (assuming the `std` feature flag is enabled).
//!
//! A common way to begin a remote debugging is connecting to a target via TCP:
//!
//! ```
//! use std::net::{TcpListener, TcpStream};
//!
//! fn wait_for_gdb_connection(port: u16) -> std::io::Result<TcpStream> {
//!     let sockaddr = format!("localhost:{}", port);
//!     eprintln!("Waiting for a GDB connection on {:?}...", sockaddr);
//!     let sock = TcpListener::bind(sockaddr)?;
//!     let (stream, addr) = sock.accept()?;
//!
//!     // Blocks until a GDB client connects via TCP.
//!     // i.e: Running `target remote localhost:<port>` from the GDB prompt.
//!
//!     eprintln!("Debugger connected from {}", addr);
//!     Ok(stream)
//! }
//! ```
//!
//! ### Creating the `GdbStub`
//!
//! All that's left is to create a new [`GdbStub`](struct.GdbStub.html), pass it
//! your `Connection` and `Target`, and call `run`!
//!
//! ```ignore
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Pre-existing setup code
//!     let mut system = Emu::new()?;
//!     // ... etc ...
//!
//!     // Establish a `Connection`
//!     let stream = wait_for_gdb_connection(9001);
//!
//!     // Create a new `GdbStub` using the established `Connection`.
//!     let debugger = GdbStub::new(stream);
//!
//!     // Instead of taking ownership of the system, GdbStub takes a &mut, yielding
//!     // ownership once the debugging session is closed, or an error occurs.
//!     let system_result = match debugger.run(&mut system) {
//!         Ok(state) => {
//!             eprintln!("Disconnected from GDB. Target state: {:?}", state);
//!             Ok(())
//!         }
//!         // handle any target-specific errors
//!         Err(gdbstub::Error::TargetError(e)) => Err(e),
//!         // connection / gdbstub internal errors
//!         Err(e) => return Err(e.into()),
//!     };
//!
//!     eprintln!("{:?}", system_result);
//! }
//! ```
//!
//! ## Real-World Examples
//!
//! There are already several projects which are using `gdbstub`:
//!
//! - [rustyboyadvance-ng](https://github.com/michelhe/rustboyadvance-ng/) -
//!   Nintendo GameBoy Advance emulator and debugger
//! - [microcorruption-emu](https://github.com/sapir/microcorruption-emu) -
//!   msp430 emulator for the microcorruption.com ctf
//! - [clicky](https://github.com/daniel5151/clicky/) - A WIP emulator for
//!   classic clickwheel iPods
//! - [ts7200](https://github.com/daniel5151/ts7200/) - An emulator for the
//!   TS-7200, a somewhat bespoke embedded ARMv4t platform
//!
//! If you happen to use `gdbstub` in one of your own projects, feel free to
//! open a PR to add it to this list!

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use core::fmt::{self, Debug};
use core::ops::Range;

use num_traits::{PrimInt, Unsigned};

mod connection_impls;
mod error;
mod protocol;
mod stub;

pub use error::Error;
pub use stub::GdbStub;

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
    type Usize: PrimInt + Unsigned + Debug + fmt::LowerHex;

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
    fn write_registers(&mut self, regs: &[u8]) -> Result<(), Self::Error>;

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

/// A trait for reading / writing bytes across some transport layer.
pub trait Connection {
    /// Transport-specific error type.
    type Error;

    /// Read a single byte.
    fn read(&mut self) -> Result<u8, Self::Error>;

    /// Write a single byte.
    fn write(&mut self, byte: u8) -> Result<(), Self::Error>;

    /// Try to read a single byte, returning None if no data is available.
    fn read_nonblocking(&mut self) -> Result<Option<u8>, Self::Error>;

    /// Read the exact number of bytes required to fill buf, blocking if
    /// necessary.
    fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), Self::Error> {
        buf.iter_mut().try_for_each(|b| {
            *b = self.read()?;
            Ok(())
        })
    }
}
