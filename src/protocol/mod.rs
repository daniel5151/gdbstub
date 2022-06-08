//! GDB protocol internals.
//!
//! These types should _not_ leak into the public interface (with a few
//! exceptions, as listed below).

mod common;
mod console_output;
mod packet;
mod response_writer;

pub(crate) mod commands;
pub(crate) mod recv_packet;

pub(crate) use common::thread_id::{ConcreteThreadId, IdKind, SpecificIdKind, SpecificThreadId};
pub(crate) use packet::Packet;
pub(crate) use response_writer::{Error as ResponseWriterError, ResponseWriter};

// These types end up a part of the public interface.
pub use console_output::ConsoleOutput;
pub use packet::PacketParseError;
