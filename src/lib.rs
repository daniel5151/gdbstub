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
//! - [Base Debugging Functionality](target/base/index.html)
//!     - Step/Continue Execution
//!     - Read/Write memory
//!     - Read/Write registers
//!     - Add/Remove Software Breakpoints
//!     - (optional) Multithreading support
//!
//! Additionally, there are many [protocol extensions](target_ext/index.html)
//! that can be optionally implemented to enhance the core debugging experience.
//! For example: if your target supports _hardware watchpoints_ (i.e: value
//! breakpoints), consider implementing the
//! [`target::ext::breakpoints::HwWatchpoint`](target_ext/breakpoints/index.
//! html) extension.
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
//!     - Log outgoing packets via `log::trace!` (using a heap-allocated output
//!       buffer)
//!
//! ## Getting Started
//!
//! This section provides a brief overview of the key traits and types used in
//! `gdbstub`, and walks though the basic steps required to integrate `gdbstub`
//! into a project.
//!
//! Additionally, I would **highly recommend** that you take a look at some of
//! the [**examples**](https://github.com/daniel5151/gdbstub/blob/master/README.md#examples)
//! listed in the project README. In particular, the included `armv4t` and
//! `armv4t_multicore` examples should serve as a good overview of what a
//! typical `gdbstub` integration might look like.
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
//! a system's execution state during a GDB debugging session, and serves as the
//! primary bridge between `gdbstub`'s generic protocol implementation and a
//! target's project/platform-specific code.
//!
//! For example: the `Target` trait includes a method called `read_registers()`,
//! which the `GdbStub` calls whenever the GDB client queries the state of the
//! target's registers.
//!
//! **`Target` is the most important trait in `gdbstub`, and must be implemented
//! by anyone who uses the library!**
//!
//! Please refer to the [`target` module documentation](target/index.html) for
//! information on how to implement `Target`.
//!
//! ### Starting the debugging session using `GdbStub`
//!
//! Once a `Connection` has been established and the `Target` has been set-up,
//! all that's left is to pass both of them over to
//! [`GdbStub`](struct.GdbStub.html) and let it do the rest!
//!
//! ```rust,ignore
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Set-up a valid `Target`
//! let mut target = MyTarget::new()?; // implements `Target`
//!
//! // Establish a `Connection`
//! let connection = wait_for_gdb_connection(9001);
//!
//! // Create a new `GdbStub` using the established `Connection`.
//! let mut debugger = GdbStub::new(connection);
//!
//! // Instead of taking ownership of the system, `GdbStub` takes a &mut, yielding
//! // ownership back to the caller once the debugging session is closed.
//! match debugger.run(&mut target) {
//!     Ok(disconnect_reason) => match disconnect_reason {
//!         DisconnectReason::Disconnect => {
//!             // run to completion
//!             while target.step() != Some(EmuEvent::Halted) {}
//!         }
//!         DisconnectReason::TargetHalted => println!("Target halted!"),
//!         DisconnectReason::Kill => {
//!             println!("GDB sent a kill command!");
//!         }
//!     }
//!     Err(GdbStubError::TargetError(e)) => {
//!         println!("Target raised a fatal error: {:?}", e);
//!     }
//!     Err(e) => return Err(e.into())
//! }
//!
//! #   Ok(())
//! # }
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
pub mod common;
pub mod target;

pub use connection::Connection;
pub use gdbstub_impl::*;

/// (Internal) The fake Tid that's used when running in single-threaded mode.
// SAFETY: 1 is clearly non-zero.
const SINGLE_THREAD_TID: common::Tid = unsafe { common::Tid::new_unchecked(1) };
/// (Internal) The fake Pid reported to GDB (since `gdbstub` only supports
/// debugging a single process).
const FAKE_PID: common::Pid = unsafe { common::Pid::new_unchecked(1) };
