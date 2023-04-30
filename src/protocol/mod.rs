//! GDB protocol internals.
//!
//! These types should _not_ leak into the public interface (with a few
//! exceptions, as listed below).

pub use console_output::ConsoleOutput;
pub use packet::PacketParseError;

mod common;
mod console_output;
mod packet;
mod response_writer;

pub(crate) mod commands;
pub(crate) mod recv_packet;
pub(crate) use common::thread_id::ConcreteThreadId;
pub(crate) use common::thread_id::IdKind;
pub(crate) use common::thread_id::SpecificIdKind;
pub(crate) use common::thread_id::SpecificThreadId;
pub(crate) use packet::Packet;
pub(crate) use response_writer::Error as ResponseWriterError;
pub(crate) use response_writer::ResponseWriter;
