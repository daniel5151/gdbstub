//! Private utility types used internally within `gdbstub`.
//!
//! These are all bits of functionality that _could_ exist as their own crates /
//! libraries, and do not rely on any `gdbstub` specific infrastructure.

pub mod managed_vec;
pub mod no_panic_iter;

pub(crate) mod dead_code_marker;
