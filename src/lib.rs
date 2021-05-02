//! An ergonomic and easy-to-integrate implementation of the
//! [GDB Remote Serial Protocol](https://sourceware.org/gdb/onlinedocs/gdb/Remote-Protocol.html#Remote-Protocol)
//! in Rust, with full `#![no_std]` support.
//!
//! ## Feature flags
//!
//! By default, the `std` and `alloc` features are enabled.
//!
//! When using `gdbstub` in `#![no_std]` contexts, make sure to set
//! `default-features = false`.
//!
//! - `alloc`
//!     - Implement `Connection` for `Box<dyn Connection>`.
//!     - Log outgoing packets via `log::trace!` (uses a heap-allocated output
//!       buffer).
//!     - Provide built-in implementations for certain protocol features:
//!         - Use a heap-allocated packet buffer in `GdbStub` (if none is
//!           provided via `GdbStubBuilder::with_packet_buffer`).
//!         - (Monitor Command) Use a heap-allocated output buffer in
//!           `ConsoleOutput`.
//! - `std` (implies `alloc`)
//!     - Implement `Connection` for [`TcpStream`](std::net::TcpStream) and
//!       [`UnixStream`](std::os::unix::net::UnixStream).
//!     - Implement [`std::error::Error`] for `gdbstub::Error`.
//!     - Add a `TargetError::Io` error variant to simplify I/O Error handling
//!       from `Target` methods.
//!
//! ## Getting Started
//!
//! This section provides a brief overview of the key traits and types used in
//! `gdbstub`, and walks though the basic steps required to integrate `gdbstub`
//! into a project.
//!
//! It is **highly recommended** to take a look at some of the
//! [**examples**](https://github.com/daniel5151/gdbstub/blob/master/README.md#examples)
//! listed in the project README. In particular, the included
//! [**`armv4t`**](https://github.com/daniel5151/gdbstub/tree/master/examples/armv4t)
//! example implements most of `gdbstub`'s protocol extensions, and can be a
//! valuable resource when getting up-and-running with `gdbstub`.
//!
//! ### The `Connection` Trait
//!
//! The [`Connection`] trait describes how `gdbstub` should communicate with the
//! main GDB process.
//!
//! `Connection` is automatically implemented for common `std` types such as
//! [`TcpStream`](std::net::TcpStream) and
//! [`UnixStream`](std::os::unix::net::UnixStream). In `#![no_std]`
//! environments, `Connection` must be manually implemented using whatever
//! in-order, serial, byte-wise I/O the hardware has available (e.g:
//! putchar/getchar over UART, an embedded TCP stack, etc.).
//!
//! A common way to start a remote debugging session is to wait for a GDB client
//! to connect via TCP:
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
//! The [`Target`](target::Target) trait describes how to control and modify
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
//! Please refer to the [`target` module documentation](target) for in-depth
//! instructions on implementing `Target`.
//!
//! ### Starting the debugging session using `GdbStub`
//!
//! Once a `Connection` has been established and `Target` has been all wired up,
//! all that's left is to hand things off to [`GdbStub`] and let it do the rest!
//!
//! ```rust,ignore
//! // Set-up a valid `Target`
//! let mut target = MyTarget::new()?; // implements `Target`
//!
//! // Establish a `Connection`
//! let connection: TcpStream = wait_for_gdb_connection(9001);
//!
//! // Create a new `GdbStub` using the established `Connection`.
//! let mut debugger = GdbStub::new(connection);
//!
//! // Instead of taking ownership of the system, `GdbStub` takes a &mut, yielding
//! // ownership back to the caller once the debugging session is closed.
//! match debugger.run(&mut target) {
//!     Ok(disconnect_reason) => match disconnect_reason {
//!         DisconnectReason::Disconnect => println!("GDB client disconnected."),
//!         DisconnectReason::TargetHalted => println!("Target halted!"),
//!         DisconnectReason::Kill => println!("GDB client sent a kill command!"),
//!     }
//!     // Handle any target-specific errors
//!     Err(GdbStubError::TargetError(e)) => {
//!         println!("Target raised a fatal error: {:?}", e);
//!         // e.g: re-enter the debugging session after "freezing" a system to
//!         // conduct some post-mortem debugging
//!         debugger.run(&mut target)?;
//!     }
//!     Err(e) => return Err(e.into())
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
