use core::fmt::{self, Debug, Display};

use crate::protocol::{PacketParseError, ResponseWriterError};
use crate::util::managed_vec::CapacityError;

/// An error which may occur during a GDB debugging session.
#[derive(Debug)]
#[non_exhaustive]
pub enum GdbStubError<T, C> {
    /// Connection Error while reading request.
    ConnectionRead(C),
    /// Connection Error while writing response.
    ConnectionWrite(C),
    /// Client nack'd the last packet, but `gdbstub` doesn't implement
    /// re-transmission.
    ClientSentNack,
    /// Packet cannot fit in the provided packet buffer.
    PacketBufferOverflow,
    /// Could not parse the packet into a valid command.
    PacketParse(PacketParseError),
    /// GDB client sent an unexpected packet. This should never happen!
    /// Please file an issue at https://github.com/daniel5151/gdbstub/issues
    PacketUnexpected,
    /// GDB client sent a packet with too much data for the given target.
    TargetMismatch,
    /// Target encountered a fatal error.
    TargetError(T),
    /// Target responded with an unsupported stop reason.
    ///
    /// Certain stop reasons can only be used when their associated protocol
    /// feature has been implemented. e.g: a Target cannot return a
    /// `StopReason::HwBreak` if the hardware breakpoints IDET hasn't been
    /// implemented.
    UnsupportedStopReason,
    /// Target didn't report any active threads when there should have been at
    /// least one running.
    NoActiveThreads,
    /// Internal - A non-fatal error occurred (with errno-style error code)
    ///
    /// This "dummy" error is required as part of the internal
    /// `TargetResultExt::handle_error()` machinery, and will never be
    /// propagated up to the end user.
    #[doc(hidden)]
    NonFatalError(u8),
}

impl<T, C> From<ResponseWriterError<C>> for GdbStubError<T, C> {
    fn from(e: ResponseWriterError<C>) -> Self {
        GdbStubError::ConnectionWrite(e.0)
    }
}

impl<A, T, C> From<CapacityError<A>> for GdbStubError<T, C> {
    fn from(_: CapacityError<A>) -> Self {
        GdbStubError::PacketBufferOverflow
    }
}

impl<T, C> Display for GdbStubError<T, C>
where
    C: Debug,
    T: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use self::GdbStubError::*;
        match self {
            ConnectionRead(e) => write!(f, "Connection Error while reading request: {:?}", e),
            ConnectionWrite(e) => write!(f, "Connection Error while writing response: {:?}", e),
            ClientSentNack => write!(f, "Client nack'd the last packet, but `gdbstub` doesn't implement re-transmission."),
            PacketBufferOverflow => write!(f, "Packet too big for provided buffer!"),
            PacketParse(e) => write!(f, "Could not parse the packet into a valid command: {:?}", e),
            PacketUnexpected => write!(f, "Client sent an unexpected packet. This should never happen! Please file an issue at https://github.com/daniel5151/gdbstub/issues"),
            TargetMismatch => write!(f, "GDB client sent a packet with too much data for the given target."),
            TargetError(e) => write!(f, "Target threw a fatal error: {:?}", e),
            UnsupportedStopReason => write!(f, "Target responded with an unsupported stop reason."),
            NoActiveThreads => write!(f, "Target didn't report any active threads when there should have been at least one running."),
            NonFatalError(_) => write!(f, "Internal - A non-fatal error occurred (with errno-style error code)"),
        }
    }
}

#[cfg(feature = "std")]
impl<T, C> std::error::Error for GdbStubError<T, C>
where
    C: Debug,
    T: Debug,
{
}
