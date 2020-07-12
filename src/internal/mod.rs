//! Internal implementation details.
//!
//! These traits / types are part of the public interface, but shouldn't be used
//! by consumers of `gdbstub` directly.

mod be_bytes;
mod maybe_unimpl;

pub use be_bytes::*;
pub use maybe_unimpl::*;
