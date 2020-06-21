use core::fmt::{self, Debug, Display};

use crate::protocol::ResponseWriterError;
use crate::util::slicevec::CapacityError;
use crate::{Connection, Target};

/// Errors which may occur during a GDB debugging session.
pub enum Error<T: Target, C: Connection> {
    /// Connection Error while reading request.
    ConnectionRead(C::Error),
    /// Connection Error while writing response.
    ConnectionWrite(ResponseWriterError<C>),
    /// Packet cannot fit in the provided packet buffer
    PacketBufferOverlow,
    /// Could not parse the packet into a valid command.
    PacketParse,
    /// Target threw a fatal error.
    TargetError(T::Error),
}

impl<T: Target, C: Connection> From<ResponseWriterError<C>> for Error<T, C> {
    fn from(e: ResponseWriterError<C>) -> Self {
        Error::ConnectionWrite(e)
    }
}

impl<A, T: Target, C: Connection> From<CapacityError<A>> for Error<T, C> {
    fn from(_: CapacityError<A>) -> Self {
        Error::PacketBufferOverlow
    }
}

impl<T: Target, C: Connection> Debug for Error<T, C>
where
    T::Error: Debug,
    C::Error: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl<T: Target, C: Connection> Display for Error<T, C>
where
    T::Error: Debug,
    C::Error: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use self::Error::*;
        match self {
            ConnectionRead(e) => write!(f, "Connection Error while reading request: {:?}", e),
            ConnectionWrite(e) => write!(f, "Connection Error while writing response: {:?}", e),
            PacketBufferOverlow => write!(f, "Packet too big for provided buffer!"),
            PacketParse => write!(f, "Could not parse the packet into a valid command."),
            TargetError(e) => write!(f, "Target threw a fatal error: {:?}", e),
        }
    }
}

#[cfg(feature = "std")]
impl<T: Target, C: Connection> std::error::Error for Error<T, C>
where
    T::Error: Debug,
    C::Error: Debug,
{
}
