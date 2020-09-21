//! Types / traits which are not expected to be directly implemented by
//! `gdbstub` users.

mod be_bytes;
mod le_bytes;

pub use be_bytes::*;
pub use le_bytes::*;

pub(crate) mod dead_code_marker;
