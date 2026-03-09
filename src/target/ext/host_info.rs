//! (LLDB extension) Provide host information to the debugger.
//!
//! This allows for reporting key-value metadata, for example the
//! target triple, endianness, and pointer size.
//!
//! This corresponds to the `qHostInfo` command in the LLDB
//! extensions.

use crate::common::Endianness;
use crate::target::Target;

/// A response key-value pair to a qHostInfo query.
///
/// A response consists of a list of key-value pairs, each of which is
/// represented by one instance of this enum.
///
/// The allowed responses are documented in the [LLDB extension
/// documentation]. Not all supported responses are currently
/// represented in this enum. If you need another one, please feel
/// free to send a PR!
///
/// [LLDB extension documentation]: https://lldb.llvm.org/resources/lldbplatformpackets.html
#[derive(Clone, Copy)]
#[non_exhaustive]
pub enum InfoResponse<'a> {
    /// The target triple for the debuggee, as a string.
    Triple(&'a str),
    /// The target endianness.
    Endianness(Endianness),
    /// The pointer size.
    PointerSize(usize),
}

/// Target Extension - Provide host information.
pub trait HostInfo: Target {
    /// Write a response to a host-info query (LLDB extension).
    ///
    /// Call `write_item` with each `InfoResponse` you wish to send.
    fn host_info(&self, write_item: &mut dyn FnMut(&InfoResponse<'_>)) -> Result<(), Self::Error>;
}

define_ext!(HostInfoOps, HostInfo);
