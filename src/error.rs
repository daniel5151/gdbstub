use alloc::string::String;
use core::fmt::{Debug, Display};

use crate::protocol::ResponseWriterError;

/// Errors which may occur during a GDB debugging session.
#[derive(Debug)]
pub enum Error<T, C> {
    /// Could not parse a packet's checksum.
    ChecksumParse,
    /// Computed checksum doesn't match packet's checksum.
    MismatchedChecksum,
    /// Connection Error.
    // TODO: rename this variant to RequestConnection
    Connection(C),
    /// Could not parse the packet into a valid command.
    // TODO: remove the `String` payload!
    PacketParse(String),
    /// Error while writing a response.
    ResponseConnection(ResponseWriterError<C>),
    /// Target threw a fatal error.
    TargetError(T),
}

impl<T, C> From<ResponseWriterError<C>> for Error<T, C> {
    fn from(e: ResponseWriterError<C>) -> Self {
        Error::ResponseConnection(e)
    }
}

impl<T: Debug, C: Debug> Display for Error<T, C> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        use self::Error::*;
        match self {
            ChecksumParse => write!(f, "Couldn't parse checksum"),
            Connection(e) => write!(f, "Connection Error: {:?}", e),
            ResponseConnection(e) => write!(f, "Connection Error while writing response: {:?}", e),
            MismatchedChecksum => write!(f, "Checksum mismatch"),
            PacketParse(e) => write!(f, "Couldn't parse command: {}", e),
            TargetError(e) => write!(f, "Target Fatal Error: {:?}", e),
        }
    }
}

#[cfg(feature = "std")]
impl<T: Debug, C: Debug> std::error::Error for Error<T, C> {}
