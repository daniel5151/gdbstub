//! (LLDB extension) Provide process information to the debugger.
//!
//! This allows for reporting key-value metadata, for example the current PID,
//! target triple, endianness, and pointer size.
//!
//! This corresponds to the `qHostInfo` command in the LLDB extensions.

use crate::common::Endianness;
use crate::common::Pid;
use crate::target::Target;

/// A response key-value pair to a [ProcessInfo::process_info] query.
///
/// A response consists of a list of key-value pairs, each of which is
/// represented by one instance of this enum.
///
/// The allowed responses are documented in the [LLDB extension documentation].
/// Not all supported responses are currently represented in this enum. If you
/// need another one, please feel free to send a PR!
///
/// [LLDB extension documentation]:
///     https://lldb.llvm.org/resources/lldbplatformpackets.html
#[derive(Clone, Copy)]
#[non_exhaustive]
pub enum ProcessInfoResponse<'a> {
    /// The current process PID.
    Pid(Pid),
    /// The target triple for the debuggee, as a string.
    Triple(&'a str),
    /// The target endianness.
    Endianness(Endianness),
    /// The pointer size.
    PointerSize(usize),
}

/// (LLDB extension) Target Extension - Provide process information.
pub trait ProcessInfo: Target {
    /// Write the response to process-info query.
    ///
    /// Call `write_item` with each `InfoResponse` you wish to send.
    fn process_info(
        &self,
        write_item: &mut dyn FnMut(&ProcessInfoResponse<'_>),
    ) -> Result<(), Self::Error>;
}

define_ext!(ProcessInfoOps, ProcessInfo);
