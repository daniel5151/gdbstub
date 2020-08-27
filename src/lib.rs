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
//! TODO: re-write this section to describe how the new trait-based modular
//! approach works.
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
//!     match debugger.run(&mut emu) {
//!         Ok(disconnect_reason) => match disconnect_reason {
//!             DisconnectReason::Disconnect => {
//!                 // run to completion
//!                 while emu.step() != Some(EmuEvent::Halted) {}
//!             }
//!             DisconnectReason::TargetHalted => println!("Target halted!"),
//!             DisconnectReason::Kill => {
//!                 println!("GDB sent a kill command!");
//!             }
//!         }
//!         Err(GdbStubError::TargetError(e)) => {
//!             println!("Emu raised a fatal error: {:?}", e);
//!         }
//!         Err(e) => return Err(e.into())
//!     }
//!
//!     Ok(())
//! }
//! ```

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(missing_docs)]

#[cfg(feature = "alloc")]
extern crate alloc;

#[macro_use]
extern crate log;

mod connection;
mod gdbstub_impl;
mod protocol;
mod util;

#[doc(hidden)]
pub mod internal;

pub mod arch;
pub mod target;

pub use connection::Connection;
pub use gdbstub_impl::*;

#[doc(inline)]
pub use target::Target;
