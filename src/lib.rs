//! An ergonomic and easy-to-integrate implementation of the
//! [GDB Remote Serial Protocol](https://sourceware.org/gdb/onlinedocs/gdb/Remote-Protocol.html#Remote-Protocol)
//! in Rust, with full `#![no_std]` support.
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
//!     - Add a `TargetError::Io` variant to simplify `std::io::Error` handling
//!       from Target methods.
//! - `paranoid_unsafe`
//!     - Please refer to the [`unsafe` in `gdbstub`](https://github.com/daniel5151/gdbstub#unsafe-in-gdbstub)
//!       section of the README.md for more details.
//!
//! ## Getting Started
//!
//! This section provides a brief overview of the key traits and types used in
//! `gdbstub`, and walks though the basic steps required to integrate `gdbstub`
//! into a project.
//!
//! At a high level, there are only three things that are required to get up and
//! running with `gdbstub`: a [`Connection`](#the-connection-trait), a
//! [`Target`](#the-target-trait), and a [event loop](#the-event-loop).
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
//! [`Connection`](conn::Connection) trait.
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
//! ## The Event Loop
//!
//! Once a [`Connection`](#the-connection-trait) has been established and the
//! [`Target`](#the-target-trait) has been initialized, all that's left is to
//! wire things up and decide what kind of event loop will be used to drive the
//! debugging session!
//!
//! First things first, let's get an instance of `GdbStub` ready to run:
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
//! ```
//!
//! Cool, but how do you actually start the debugging session?
// use an explicit doc attribute to avoid automatic rustfmt wrapping
#![doc = "### `GdbStub::run_blocking`: The quick and easy way to get up and running with `gdbstub`"]
//!
//! If you're running on a hosted system and have an extra thread to spare, the
//! quickest way to get up and running with `gdbstub` is by using the
//! [`GdbStub::run_blocking`](stub::run_blocking) API alongside the
//! [`BlockingEventLoop`] trait.
//!
//! If you are on a more resource constrained platform, and/or don't wish to
//! dedicate an entire thread to `gdbstub`, feel free to skip ahead to the
//! [following
//! section](#gdbstubstatemachine-driving-gdbstub-in-an-async-event-loop--via-interrupt-handlers).
//!
//!
//! A basic integration might look something like this:
//!
//! ```rust
//! # use gdbstub::target::ext::base::BaseOps;
//! #
//! # struct MyTarget;
//! #
//! # impl Target for MyTarget {
//! #     type Error = &'static str;
//! #     type Arch = gdbstub_arch::arm::Armv4t; // as an example
//! #     fn base_ops(&mut self) -> BaseOps<Self::Arch, Self::Error> { todo!() }
//! # }
//! #
//! # impl MyTarget {
//! #     fn run_and_check_for_incoming_data(
//! #         &mut self,
//! #         conn: &mut impl Connection
//! #     ) -> MyTargetEvent { todo!() }
//! #
//! #     fn stop_in_response_to_ctrl_c_interrupt(
//! #         &mut self
//! #     ) -> Result<(), &'static str> { todo!() }
//! # }
//! #
//! # enum MyTargetEvent {
//! #     IncomingData,
//! #     StopReason(SingleThreadStopReason<u32>),
//! # }
//! #
//! use gdbstub::common::Signal;
//! use gdbstub::conn::{Connection, ConnectionExt}; // note the use of `ConnectionExt`
//! use gdbstub::stub::{run_blocking, DisconnectReason, GdbStub, GdbStubError};
//! use gdbstub::stub::SingleThreadStopReason;
//! use gdbstub::target::Target;
//!
//! enum MyGdbBlockingEventLoop {}
//!
//! impl run_blocking::BlockingEventLoop for MyGdbBlockingEventLoop {
//!     type Target = MyTarget;
//!     type Connection = Box<dyn ConnectionExt<Error = std::io::Error>>;
//!
//!     // or MultiThreadStopReason on multi threaded targets
//!     type StopReason = SingleThreadStopReason<u32>;
//!
//!     /// Invoked immediately after the target's `resume` method has been
//!     /// called. The implementation should block until either the target
//!     /// reports a stop reason, or if new data was sent over the connection.
//!     fn wait_for_stop_reason(
//!         target: &mut MyTarget,
//!         conn: &mut Self::Connection,
//!     ) -> Result<
//!         run_blocking::Event<SingleThreadStopReason<u32>>,
//!         run_blocking::WaitForStopReasonError<
//!             <Self::Target as Target>::Error,
//!             <Self::Connection as Connection>::Error,
//!         >,
//!     > {
//!         // the specific mechanism to "select" between incoming data and target
//!         // events will depend on your project's architecture.
//!         //
//!         // some examples of how you might implement this method include: `epoll`,
//!         // `select!` across multiple event channels, periodic polling, etc...
//!         //
//!         // in this example, lets assume the target has a magic method that handles
//!         // this for us.
//!         let event = match target.run_and_check_for_incoming_data(conn) {
//!             MyTargetEvent::IncomingData => {
//!                 let byte = conn
//!                     .read() // method provided by the `ConnectionExt` trait
//!                     .map_err(run_blocking::WaitForStopReasonError::Connection)?;
//!
//!                 run_blocking::Event::IncomingData(byte)
//!             }
//!             MyTargetEvent::StopReason(reason) => {
//!                 run_blocking::Event::TargetStopped(reason)
//!             }
//!         };
//!
//!         Ok(event)
//!     }
//!
//!     /// Invoked when the GDB client sends a Ctrl-C interrupt. The
//!     /// implementation should handle the interrupt request + return an
//!     /// appropriate stop reason to report back to the GDB client, or return
//!     /// `None` if the interrupt should be ignored.
//!     fn on_interrupt(
//!         target: &mut MyTarget,
//!     ) -> Result<Option<SingleThreadStopReason<u32>>, <MyTarget as Target>::Error> {
//!         // notify the target that a ctrl-c interrupt has occurred.
//!         target.stop_in_response_to_ctrl_c_interrupt()?;
//!
//!         // a pretty typical stop reason in response to a Ctrl-C interrupt is to
//!         // report a "Signal::SIGINT".
//!         Ok(Some(SingleThreadStopReason::Signal(Signal::SIGINT).into()))
//!     }
//! }
//!
//! fn gdb_event_loop_thread(
//!     debugger: GdbStub<MyTarget, Box<dyn ConnectionExt<Error = std::io::Error>>>,
//!     mut target: MyTarget
//! ) {
//!     match debugger.run_blocking::<MyGdbBlockingEventLoop>(&mut target) {
//!         Ok(disconnect_reason) => match disconnect_reason {
//!             DisconnectReason::Disconnect => {
//!                 println!("Client disconnected")
//!             }
//!             DisconnectReason::TargetExited(code) => {
//!                 println!("Target exited with code {}", code)
//!             }
//!             DisconnectReason::TargetTerminated(sig) => {
//!                 println!("Target terminated with signal {}", sig)
//!             }
//!             DisconnectReason::Kill => println!("GDB sent a kill command"),
//!         },
//!         Err(GdbStubError::TargetError(e)) => {
//!             println!("target encountered a fatal error: {}", e)
//!         }
//!         Err(e) => {
//!             println!("gdbstub encountered a fatal error: {}", e)
//!         }
//!     }
//! }
//! ```
// use an explicit doc attribute to avoid automatic rustfmt wrapping
#![doc = "### `GdbStubStateMachine`: Driving `gdbstub` in an async event loop / via interrupt handlers"]
//!
//! `GdbStub::run_blocking` requires that the target implement the
//! [`BlockingEventLoop`] trait, which as the name implies, uses _blocking_ IO
//! when handling certain events. Blocking the thread is a totally reasonable
//! approach in most implementations, as one can simply spin up a separate
//! thread to run the GDB stub (or in certain emulator implementations, run the
//! emulator as part of the `wait_for_stop_reason` method).
//!
//! Unfortunately, this blocking behavior can be a non-starter when integrating
//! `gdbstub` in projects that don't support / wish to avoid the traditional
//! thread-based execution model, such as projects using `async/await`, or
//! bare-metal, `no_std` projects running on embedded hardware.
//!
//! In these cases, `gdbstub` provides access to the underlying
//! [`GdbStubStateMachine`] API, which gives implementations full control over
//! the GDB stub's "event loop". This API requires implementations to "push"
//! data to the `gdbstub` implementation whenever new data becomes available
//! (e.g: when a UART interrupt handler receives a byte, when the target hits a
//! breakpoint, etc...), as opposed to the `GdbStub::run_blocking` API, which
//! "pulls" these events in a blocking manner.
//!
//! See the [`GdbStubStateMachine`] docs for more details on how to use this
//! API.
//!
//! [`GdbStubStateMachine`]: stub::state_machine::GdbStubStateMachine
//! [`BlockingEventLoop`]: stub::run_blocking::BlockingEventLoop

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(missing_docs)]
#![deny(rust_2018_idioms, future_incompatible, nonstandard_style)]
// Primarily due to rust-lang/rust#8995
//
// If this ever gets fixed, it's be possible to rewrite complex types using inherent associated type
// aliases.
//
// For example, instead of writing this monstrosity:
//
// Result<Option<MultiThreadStopReason<<Self::Arch as Arch>::Usize>>, Self::Error>
//
// ...it could be rewritten as:
//
// type StopReason = MultiThreadStopReason<<Self::Arch as Arch>::Usize>>;
//
// Result<Option<StopReason>, Self::Error>
#![allow(clippy::type_complexity)]

#[cfg(feature = "alloc")]
extern crate alloc;

#[macro_use]
extern crate log;

mod protocol;
mod util;

#[doc(hidden)]
pub mod internal;

pub mod arch;
pub mod common;
pub mod conn;
pub mod stub;
pub mod target;

/// (Internal) The fake Tid that's used when running in single-threaded mode.
// SAFETY: 1 is clearly non-zero.
const SINGLE_THREAD_TID: common::Tid = unsafe { common::Tid::new_unchecked(1) };
/// (Internal) The fake Pid reported to GDB (since `gdbstub` only supports
/// debugging a single process).
const FAKE_PID: common::Pid = unsafe { common::Pid::new_unchecked(1) };

pub(crate) mod is_valid_tid {
    pub trait IsValidTid {}

    impl IsValidTid for () {}
    impl IsValidTid for crate::common::Tid {}
}
