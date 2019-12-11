use alloc::string::String;
use core::fmt::{Debug, Display};

use crate::protocol::ResponseWriterError;

#[derive(Debug)]
pub enum Error<T, C> {
    ChecksumParse,
    Connection(C),
    ResponseConnection(ResponseWriterError<C>),
    MismatchedChecksum,
    PacketParse(String),
    TargetError(T),
    Unexpected,
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
            Unexpected => write!(f, "Client sent an unexpected packet"),
        }
    }
}

#[cfg(feature = "std")]
impl<T: Debug, C: Debug> std::error::Error for Error<T, C> {}
