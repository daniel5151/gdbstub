//! Community-created implementations of [`gdbstub::arch::Arch`] for various
//! architectures.
//!
//! _Note:_ If an architecture is missing from this crate, that does _not_ mean
//! that it can't be used with `gdbstub`! So-long as there's support for the
//! target architecture in GDB, it should be fairly straightforward to implement
//! `Arch` manually.
//!
//! Please consider upstreaming any missing `Arch` implementations you happen to
//! implement yourself! Aside from the altruistic motive of improving `gdbstub`,
//! upstreaming your `Arch` implementation will ensure that it's kept up-to-date
//! with any future breaking API changes.
//!
//! **Disclaimer:** These implementations are all community contributions, and
//! while they are tested (by the PR's author) and code-reviewed, it's not
//! particularly feasible to write detailed tests for each architecture! If you
//! spot a bug in any of the implementations, please file an issue / open a PR!
//!
//! # What's with `RegIdImpl`?
//!
//! Supporting the `Target::read/write_register` API required introducing a new
//! [`RegId`] trait + [`Arch::RegId`] associated type. `RegId` is used by
//! `gdbstub` to translate raw GDB register ids (a protocol level arch-dependent
//! `usize`) into human-readable enum variants.
//!
//! Unfortunately, this API was added after several contributors had already
//! upstreamed their `Arch` implementations, and as a result, there are several
//! built-in arch implementations which are missing proper `RegId` enums
//! (tracked under [issue #29](https://github.com/daniel5151/gdbstub/issues/29)).
//!
//! As a stop-gap measure, affected `Arch` implementations have been modified to
//! accept a `RegIdImpl` type parameter, which requires users to manually
//! specify a `RegId` implementation.
//!
//! If you're not interested in implementing the `Target::read/write_register`
//! methods and just want to get up-and-running with `gdbstub`, it's fine to
//! set `RegIdImpl` to `()` and use the built-in stubbed `impl RegId for ()`.
//!
//! A better approach would be to implement (and hopefully upstream!) a proper
//! `RegId` enum. While this will require doing a bit of digging through the GDB
//! docs + [architecture XML definitions](https://github.com/bminor/binutils-gdb/tree/master/gdb/features/),
//! it's not too tricky to get a working implementation up and running, and
//! makes it possible to safely and efficiently implement the
//! `Target::read/write_register` API. As an example, check out
//! [`ArmCoreRegId`](arm::reg::id::ArmCoreRegId#impl-RegId).
//!
//! Whenever a `RegId` enum is upstreamed, the associated `Arch`'s `RegIdImpl`
//! parameter will be defaulted to the newly added enum. This will simplify the
//! API without requiring an explicit breaking API change. Once all `RegIdImpl`
//! have a default implementation, only a single breaking API change will be
//! required to remove `RegIdImpl` entirely (along with this documentation).

#![cfg_attr(not(test), no_std)]
#![deny(missing_docs)]

pub mod arm;
pub mod mips;
pub mod msp430;
pub mod ppc;
pub mod riscv;
pub mod x86;

// used as part of intra-doc link
#[allow(unused_imports)]
use gdbstub::arch::*;
