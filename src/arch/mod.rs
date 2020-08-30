//! Built-in implementations of [`Arch`](../trait.Arch.html) for various
//! architectures.
//!
//! _Note:_ If an architecture is missing from this module, that does _not_ mean
//! that it can't be used with `gdbstub`! So-long as there's support for the
//! target architecture in GDB, it should be fairly straightforward to implement
//! `Arch` manually.
//!
//! Please consider upstreaming any missing `Arch` implementations you happen to
//! implement yourself!

pub mod arm;
pub mod mips;
pub mod msp430;
pub mod riscv;
mod traits;
pub mod x86;

pub use traits::*;
