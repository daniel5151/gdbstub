//! An ergonomic and easy-to-integrate implementation of the
//! [GDB Remote Serial Protocol](https://sourceware.org/gdb/onlinedocs/gdb/Remote-Protocol.html#Remote-Protocol)
//! in Rust.
//!
//! `gdbstub` is entirely `#![no_std]` compatible, and can be used on platforms
//! without a global allocator. In embedded contexts, `gdbstub` can be
//! configured to use pre-allocated buffers and communicate over any available
//! serial I/O connection (e.g: UART).
//!
//! `gdbstub` is particularly well suited for _emulation_, making it easy to add
//! powerful, non-intrusive debugging support to an emulated system. Just
//! provide an implementation of [`Target`](trait.Target.html) for your target
//! platform, and you're ready to start debugging!
//!
//! ## Debugging Features
//!
//! Features marked as (optional) aren't required to be implemented, but can be
//! implemented to enhance the debugging experience.
//!
//! - Core GDB Protocol
//!     - Step + Continue
//!     - Add + Remove Software Breakpoints
//!     - Read/Write memory
//!     - Read/Write registers
//!     - (optional) Add + Remove Hardware Breakpoints
//!     - (optional) Read/Write/Access Watchpoints (i.e: value breakpoints)
//!     - (optional) Multithreading support
//! - Extended GDB Protocol
//!     - (optional) Handle custom debug commands (sent via GDB's `monitor`
//!       command)
//!     - (optional) Automatic architecture detection
//!
//! If `gdbstub` is missing a feature you'd like to use, please file an issue /
//! open a PR!
//!
//! ## Feature flags
//!
//! The `std` feature is enabled by default. In `#![no_std]` contexts, use
//! `default-features = false`.
//!
//! - `alloc`
//!     - Implements `Connection` for `Box<dyn Connection>`.
//!     - Adds output buffering to `ConsoleOutput`.
//! - `std` (implies `alloc`)
//!     - Implements `Connection` for [`TcpStream`](https://doc.rust-lang.org/std/net/struct.TcpStream.html)
//!       and [`UnixStream`](https://doc.rust-lang.org/std/os/unix/net/struct.UnixStream.html).
//!     - Implements [`std::error::Error`](https://doc.rust-lang.org/std/error/trait.Error.html)
//!       for `gdbstub::Error`
//!     - Log outgoing packets via `log::trace!` (uses a heap-allocated output
//!       buffer)
//!
//! ## Getting Started
//!
//! This section provides a brief overview of the key traits and types used in
//! `gdbstub`, and walks though the basic steps required to integrate `gdbstub`
//! into a project.
//!
//! Additionally, if you're looking for some more fleshed-out examples, take a
//! look at some of the [examples](https://github.com/daniel5151/gdbstub/blob/master/README.md#examples)
//! listed in the project README.
//!
//! ### The `Connection` Trait
//!
//! The [`Connection`](trait.Connection.html) trait describes how `gdbstub`
//! should communicate with the main GDB process.
//!
//! `Connection` is automatically implemented for common `std` types such as
//! `TcpStream` and `UnixStream`. In `#![no_std]` environments, `Connection`
//! must be implemented manually, using whatever bytewise transport the
//! hardware has available (e.g: UART).
//!
//! A common way to start a remote debugging session is to wait for the GDB
//! client to connect via TCP:
//!
//! ```rust
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
//! ### The `Target` Trait
//!
//! The [`Target`](trait.Target.html) trait describes how to control and modify
//! a system's execution state during a GDB debugging session. Since each target
//! is different, it's up to the user to provide methods to read/write memory,
//! start/stop execution, etc...
//!
//! One key ergonomic feature of the `Target` trait is that it "plumbs-through"
//! any existing project-specific error-handling via the `Target::Error`
//! associated type. Every method of `Target` returns a `Result<T,
//! Target::Error>`, which makes it's possible to use the `?` operator for error
//! handling, _without_ having wrapping errors in `gdbstub` specific variants!
//!
//! For example, here's what an implementation of `Target` might look like for a
//! single-core emulator targeting the ARMv4T instruction set. See the
//! [examples](https://github.com/daniel5151/gdbstub/blob/master/README.md#examples)
//! section of the project README for more fleshed-out examples.
//!
//! ```rust,ignore
//! // Simplified and modified from gdbstub/examples/armv4t/gdb.rs
//!
//! use gdbstub::{
//!     arch, BreakOp, ResumeAction, StopReason, Target, Tid, TidSelector,
//!     SINGLE_THREAD_TID,
//! };
//!
//! // ------------- Existing Emulator Code ------------- //
//!
//! enum EmuError {
//!     BadRead,
//!     BadWrite,
//!     // ...
//! }
//!
//! struct Emu {
//!     breakpoints: Vec<u32>,
//!     /* ... */
//! }
//! impl Emu {
//!     fn step(&mut self) -> Result<Option<EmuEvent>, EmuError>;
//!     fn read8(&mut self, addr: u32) -> Result<u8, EmuError>;
//!     fn write8(&mut self, addr: u32, val: u8) -> Result<(), EmuError>;
//! }
//!
//! enum EmuEvent {
//!     Halted,
//!     Break
//! }
//!
//! // ------------- `gdbstub` Integration ------------- //
//!
//! impl Target for Emu {
//!     type Arch = arch::arm::Armv4t;
//!     type Error = EmuError;
//!
//!     fn resume(
//!         &mut self,
//!         actions: &mut dyn Iterator<Item = (TidSelector, ResumeAction)>,
//!         check_gdb_interrupt: &mut dyn FnMut() -> bool,
//!     ) -> Result<(Tid, StopReason<u32>), Self::Error> {
//!         // one thread, only one action
//!         let (_, action) = actions.next().unwrap();
//!
//!         let event = match action {
//!             ResumeAction::Step => match self.step()? {
//!                 Some(e) => e,
//!                 None => return Ok((SINGLE_THREAD_TID, StopReason::DoneStep)),
//!             },
//!             ResumeAction::Continue => {
//!                 let mut cycles = 0;
//!                 loop {
//!                     if let Some(event) = self.step()? {
//!                         break event;
//!                     };
//!
//!                     // check for GDB interrupt every 1024 instructions
//!                     cycles += 1;
//!                     if cycles % 1024 == 0 && check_gdb_interrupt() {
//!                         return Ok((SINGLE_THREAD_TID, StopReason::GdbInterrupt));
//!                     }
//!                 }
//!             }
//!         };
//!
//!         Ok((
//!             SINGLE_THREAD_TID,
//!             match event {
//!                 EmuEvent::Halted => StopReason::Halted,
//!                 EmuEvent::Break => StopReason::HwBreak,
//!             },
//!         ))
//!     }
//!
//!     fn read_registers(
//!         &mut self,
//!         regs: &mut arch::arm::reg::ArmCoreRegs,
//!     ) -> Result<(), EmuError> {
//!         // fill up `regs` be querying self
//!         Ok(())
//!     }
//!
//!     fn write_registers(&mut self, regs: &arch::arm::reg::ArmCoreRegs) -> Result<(), EmuError> {
//!         // update `self` with data from `regs`
//!         Ok(())
//!     }
//!
//!     fn read_addrs(
//!         &mut self,
//!         addr: std::ops::Range<u32>,
//!         push_byte: &mut dyn FnMut(u8),
//!     ) -> Result<(), EmuError> {
//!         for addr in addr {
//!             push_byte(self.read8(addr)?)
//!         }
//!         Ok(())
//!     }
//!
//!     fn write_addrs(&mut self, start_addr: u32, data: &[u8]) -> Result<(), EmuError> {
//!         for (addr, val) in (start_addr..).zip(data.iter().copied()) {
//!             self.write8(addr, val)?
//!         }
//!         Ok(())
//!     }
//!
//!     fn update_sw_breakpoint(&mut self, addr: u32, op: BreakOp) -> Result<bool, EmuError> {
//!         match op {
//!             BreakOp::Add => self.breakpoints.push(addr),
//!             BreakOp::Remove => {
//!                 let pos = match self.breakpoints.iter().position(|x| *x == addr) {
//!                     None => return Ok(false),
//!                     Some(pos) => pos,
//!                 };
//!                 self.breakpoints.remove(pos);
//!             }
//!         }
//!
//!         Ok(true)
//!     }
//! }
//! ```
//!
//! ### Starting the debugging session
//!
//! Once a `Connection` has been established and a `Target` is available, all
//! that's left is to pass both of them over to
//! [`GdbStub`](struct.GdbStub.html) and let it do the rest!
//!
//! ```rust,ignore
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Pre-existing setup code
//!     let mut emu = Emu::new()?;
//!     // ... etc ...
//!
//!     // Establish a `Connection`
//!     let connection = wait_for_gdb_connection(9001);
//!
//!     // Create a new `GdbStub` using the established `Connection`.
//!     let debugger = GdbStub::new(connection);
//!
//!     // Instead of taking ownership of the system, GdbStub takes a &mut, yielding
//!     // ownership once the debugging session is closed, or an error occurs.
//!     match debugger.run(&mut emu)? {
//!         DisconnectReason::Disconnect => {
//!             // run to completion
//!             while emu.step() != Some(EmuEvent::Halted) {}
//!         }
//!         DisconnectReason::TargetHalted => println!("Target halted!"),
//!         DisconnectReason::Kill => {
//!             println!("GDB sent a kill command!");
//!             return Ok(());
//!         }
//!     }
//! }
//! ```

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(missing_docs)]

#[cfg(feature = "alloc")]
extern crate alloc;

#[macro_use]
extern crate log;

pub mod arch;

mod arch_traits;
mod connection;
mod error;
mod gdbstub;
pub mod opt_result_impl;
mod protocol;
mod target;
mod util;

pub use arch_traits::{Arch, Registers};
pub use connection::Connection;
pub use error::Error;
pub use gdbstub::*;
pub use protocol::{ConsoleOutput, TidSelector};
pub use target::*;
pub use util::be_bytes::BeBytes;

/// Thread ID
// TODO: FUTURE: expose full PID.TID to client?
pub type Tid = core::num::NonZeroUsize;

/// TID returned by `Target::resume` on single-threaded systems.
// SAFETY: 1 is a non-zero value :P
pub const SINGLE_THREAD_TID: Tid = unsafe { Tid::new_unchecked(1) };

/// A result type which includes an "unimplemented" state.
///
/// `OptResult<T, E>` should be indistinguishable from `Result<T, E>`, aside
/// from the small caveat of having to use `.into()` when returning `Err`
/// variants (i.e: `return Err(foo)` will fail to compile).
pub type OptResult<T, E> = Result<T, opt_result_impl::MaybeNoImpl<E>>;
