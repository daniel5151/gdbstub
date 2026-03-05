//! Provide host and process information to the debugger.
//!
//! These correspond to the `qHostInfo` and `qProcessInfo` commands.
//! They report key-value metadata such as the target triple,
//! endianness, pointer size, and process ID.
//!
//! The callback passed to these methods should be called with byte slices
//! that together form a semicolon-delimited `key:value;` response string.
//! For example:
//!
//! ```text
//! triple:7761736d33322d756e6b6e6f776e2d756e6b6e6f776e2d7761736d;pid:1;endian:little;ptrsize:4;
//! ```
//!
//! Note: the `triple` value must be hex-encoded.

use crate::target::Target;

/// Target Extension - Provide host and process information.
pub trait ProcessInfo: Target {
    /// Write the response to `qHostInfo`.
    ///
    /// Call `write` one or more times with byte slices that together form
    /// the response. Each call appends to the output.
    fn host_info(&self, write: &mut dyn FnMut(&[u8])) -> Result<(), Self::Error>;

    /// Write the response to `qProcessInfo`.
    ///
    /// Call `write` one or more times with byte slices that together form
    /// the response. Each call appends to the output.
    fn process_info(&self, write: &mut dyn FnMut(&[u8])) -> Result<(), Self::Error>;
}

define_ext!(ProcessInfoOps, ProcessInfo);
