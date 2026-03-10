//! (LLDB extension) Provide host information to the debugger.
//!
//! This allows for reporting key-value metadata, for example the
//! target triple, endianness, and pointer size.
//!
//! This corresponds to the `qHostInfo` command in the LLDB
//! extensions.

use crate::common::Endianness;
use crate::target::Target;

/// A response key-value pair to a [HostInfo::host_info] query.
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
pub enum HostInfoResponse<'a> {
    /// The target triple for the debuggee, as a string.
    Triple(&'a str),
    /// The target endianness.
    Endianness(Endianness),
    /// The pointer size.
    PointerSize(usize),
}

/// (LLDB extension) Target Extension - Provide host information.
pub trait HostInfo: Target {
    /// Write a response to a host-info query.
    ///
    /// Call `write_item` with each `HostInfoResponse` you wish to send.
    fn host_info(
        &self,
        write_item: &mut dyn FnMut(&HostInfoResponse<'_>),
    ) -> Result<(), Self::Error>;
}

define_ext!(HostInfoOps, HostInfo);
