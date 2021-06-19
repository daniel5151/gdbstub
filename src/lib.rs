//! An ergonomic and easy-to-integrate implementation of the
//! [GDB Remote Serial Protocol](https://sourceware.org/gdb/onlinedocs/gdb/Remote-Protocol.html#Remote-Protocol)
//! in Rust, with full `#![no_std]` support.
//!
//! ## Getting Started
//!
//! This section provides a brief overview of the key traits and types used in
//! `gdbstub`, and walks though the basic steps required to integrate `gdbstub`
//! into a project.
//!
//! At a high level, there are only two things that are required to get up and
//! running with `gdbstub`: a [`Connection`](#the-connection-trait), and a
//! [`Target`](#the-target-trait)
//!
//! > _Note:_ I _highly recommended_ referencing some of the
//! [examples](https://github.com/daniel5151/gdbstub/blob/master/README.md#examples)
//! listed in the project README when integrating `gdbstub` into a project for
//! the first time.
//!
//! > In particular, the in-tree
//! [`armv4t`](https://github.com/daniel5151/gdbstub/tree/master/examples/armv4t)
//! example contains basic implementations off almost all protocol extensions,
//! making it an incredibly valuable reference when implementing protocol
//! extensions.
//!
//! ### The `Connection` Trait
//!
//! First things first: `gdbstub` needs some way to communicate with a GDB
//! client. To facilitate this communication, `gdbstub` uses a custom
//! [`Connection`] trait.
//!
//! `Connection` is automatically implemented for common `std` types such as
//! [`TcpStream`](std::net::TcpStream) and
//! [`UnixStream`](std::os::unix::net::UnixStream).
//!
//! If you're using `gdbstub` in a `#![no_std]` environment, `Connection` will
//! most likely need to be manually implemented on top of whatever in-order,
//! serial, byte-wise I/O your particular platform has available (e.g:
//! putchar/getchar over UART, using an embedded TCP stack, etc.).
//!
//! One common way to start a remote debugging session is to simply wait for a
//! GDB client to connect via TCP:
//!
//! ```rust
//! use std::io;
//! use std::net::{TcpListener, TcpStream};
//!
//! fn wait_for_gdb_connection(port: u16) -> io::Result<TcpStream> {
//!     let sockaddr = format!("localhost:{}", port);
//!     eprintln!("Waiting for a GDB connection on {:?}...", sockaddr);
//!     let sock = TcpListener::bind(sockaddr)?;
//!     let (stream, addr) = sock.accept()?;
//!
//!     // Blocks until a GDB client connects via TCP.
//!     // i.e: Running `target remote localhost:<port>` from the GDB prompt.
//!
//!     eprintln!("Debugger connected from {}", addr);
//!     Ok(stream) // `TcpStream` implements `gdbstub::Connection`
//! }
//! ```
//!
//! ### The `Target` Trait
//!
//! The [`Target`](target::Target) trait describes how to control and modify
//! a system's execution state during a GDB debugging session, and serves as the
//! primary bridge between `gdbstub`'s generic GDB protocol implementation and a
//! specific target's project/platform-specific code.
//!
//! At a high level, the `Target` trait is a collection of user-defined handler
//! methods that the GDB client can invoke via the GDB remote serial protocol.
//! For example, the `Target` trait includes methods to read/write
//! registers/memory, start/stop execution, etc...
//!
//! **`Target` is the most important trait in `gdbstub`, and must be implemented
//! by anyone integrating `gdbstub` into their project!**
//!
//! Please refer to the [`target` module documentation](target) for in-depth
//! instructions on how to implement [`Target`](target::Target) for a particular
//! platform.
//!
//! ### Starting the debugging session using `GdbStub`
//!
//! Once a [`Connection`](#the-connection-trait) has been established and
//! [`Target`](#the-target-trait) has been all wired up, all that's left is to
//! hand things off to [`gdbstub::GdbStub`](GdbStub) and let it do the rest!
//!
//! ```rust,ignore
//! // Set-up a valid `Target`
//! let mut target = MyTarget::new()?; // implements `Target`
//!
//! // Establish a `Connection`
//! let connection: TcpStream = wait_for_gdb_connection(9001);
//!
//! // Create a new `gdbstub::GdbStub` using the established `Connection`.
//! let mut debugger = gdbstub::GdbStub::new(connection);
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
//!         // `gdbstub` will not immediate close the debugging session if a
//!         // fatal error occurs, enabling "post mortem" debugging if required.
//!         debugger.run(&mut target)?;
//!     }
//!     Err(e) => return Err(e.into())
//! }
//! ```
//!
//! ## Feature flags
//!
//! By default, both the `std` and `alloc` features are enabled.
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

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(missing_docs)]
// Primarily due to rust-lang/rust#8995
//
// If this ever gets fixed, it's be possible to rewrite complex types using inherent associated type
// aliases.
//
// For example, instead of writing this monstrosity:
//
// Result<Option<ThreadStopReason<<Self::Arch as Arch>::Usize>>, Self::Error>
//
// ...it could be rewritten as:
//
// type StopReason = ThreadStopReason<<Self::Arch as Arch>::Usize>>;
//
// Result<Option<StopReason>, Self::Error>
#![allow(clippy::type_complexity)]

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

pub use connection::{Connection, ConnectionExt};
pub use gdbstub_impl::*;

/// (Internal) The fake Tid that's used when running in single-threaded mode.
// SAFETY: 1 is clearly non-zero.
const SINGLE_THREAD_TID: common::Tid = unsafe { common::Tid::new_unchecked(1) };
/// (Internal) The fake Pid reported to GDB (since `gdbstub` only supports
/// debugging a single process).
const FAKE_PID: common::Pid = unsafe { common::Pid::new_unchecked(1) };
