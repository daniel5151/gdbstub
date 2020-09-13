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
//!
//! **Disclaimer:** These implementations are all community contributions, and
//! while they are tested (by the PR's author) and code-reviewed, it's not
//! particularly feasible to write detailed tests for each architecture! If you
//! spot a bug in any of the implementations, please file an issue / open a PR!

pub mod arm;
pub mod mips;
pub mod msp430;
pub mod ppc;
pub mod riscv;
mod traits;
pub mod x86;

pub use traits::*;
