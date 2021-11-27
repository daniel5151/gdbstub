//! Types / traits which are part of `gdbstub`'s public API, but don't need to
//! be implemented by consumers of the library.

mod be_bytes;
mod le_bytes;

pub use be_bytes::*;
pub use le_bytes::*;
