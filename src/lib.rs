//! An implementation of the
//! [GDB Remote Serial Protocol](https://sourceware.org/gdb/onlinedocs/gdb/Remote-Protocol.html#Remote-Protocol)
//! in Rust.
//!
//! ***TODO BEFORE PUBLISHING: *** re-write these docs with the new interface!
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
