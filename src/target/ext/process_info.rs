//! Provide host and process information to the debugger.
//!
//! These correspond to the `qHostInfo` and `qProcessInfo` commands.
//! They report key-value metadata such as the target triple,
//! endianness, pointer size, and process ID. We take the information
//! that we can return as responses to this commands in the
//! [`HostInfoResponse`] and [`ProcessInfoResponse`] structs,
//! respectively.

use crate::common::{Endianness, Pid};

/// A response key-value pair to a qProcessInfo or qHostInfo query.
///
/// A response to either of these commands consists of a list of
/// key-value pairs, each of which is represented by one instance of
/// this enum.
pub enum InfoResponse<'a> {
    /// The current process PID.
    Pid(Pid),
    /// The target triple for the debuggee, as a string.
    Triple(&'a str),
    /// The target endianness.
    Endianness(Endianness),
    /// The pointer size.
    PointerSize(usize),
}

use crate::target::Target;

/// Target Extension - Provide host and process information.
pub trait ProcessInfo: Target {
    /// Write a response to `qHostInfo`.
    ///
    /// Call `write_item` with each `InfoResponse` you wish to send.
    fn host_info(&self, write_item: &mut dyn FnMut(&InfoResponse<'_>)) -> Result<(), Self::Error>;

    /// Write the response to `qProcessInfo`.
    ///
    /// Call `write_item` with each `InfoResponse` you wish to send.
    fn process_info(
        &self,
        write_item: &mut dyn FnMut(&InfoResponse<'_>),
    ) -> Result<(), Self::Error>;
}

define_ext!(ProcessInfoOps, ProcessInfo);
