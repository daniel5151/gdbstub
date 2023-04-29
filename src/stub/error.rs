use core::fmt::{self, Debug, Display};

use crate::protocol::{PacketParseError, ResponseWriterError};
use crate::util::managed_vec::CapacityError;

/// An error which may occur during a GDB debugging session.
#[derive(Debug)]
#[non_exhaustive]
pub enum GdbStubError<T, C> {
    /// Connection Error while initializing the session.
    ConnectionInit(C),
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
    /// Please re-run with `log` trace-level logging enabled and file an issue
    /// at <https://github.com/daniel5151/gdbstub/issues>
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
    /// GDB client sent an unexpected `step` request, most-likely due to this
    /// GDB client bug: <https://sourceware.org/bugzilla/show_bug.cgi?id=28440>.
    ///
    /// Unfortunately, there's nothing `gdbstub` can do to work around this bug,
    /// so if you've encountered this error, you'll need to implement
    /// single-step support for your target.
    UnexpectedStepPacket,

    /// The target has not opted into using implicit software breakpoints.
    /// See [`Target::guard_rail_implicit_sw_breakpoints`] for more information.
    ///
    /// [`Target::guard_rail_implicit_sw_breakpoints`]:
    /// crate::target::Target::guard_rail_implicit_sw_breakpoints
    ImplicitSwBreakpoints,
    /// GDB client attempted to attach to a new process, but the target has not
    /// implemented [`ExtendedMode::support_current_active_pid`].
    ///
    /// [`ExtendedMode::support_current_active_pid`]:
    ///     crate::target::ext::extended_mode::ExtendedMode::support_current_active_pid
    // DEVNOTE: this is a temporary workaround for something that can and should
    // be caught at compile time via IDETs. That said, since i'm not sure when
    // I'll find the time to cut a breaking release of gdbstub, I'd prefer to
    // push out this feature as a non-breaking change now.
    MissingCurrentActivePidImpl,

    // Internal - A non-fatal error occurred (with errno-style error code)
    //
    // This "dummy" error is required as part of the internal
    // `TargetResultExt::handle_error()` machinery, and will never be
    // propagated up to the end user.
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
            ConnectionInit(e) => write!(f, "Connection Error while initializing the session: {:?}", e),
            ConnectionRead(e) => write!(f, "Connection Error while reading request: {:?}", e),
            ConnectionWrite(e) => write!(f, "Connection Error while writing response: {:?}", e),
            ClientSentNack => write!(f, "Client nack'd the last packet, but `gdbstub` doesn't implement re-transmission."),
            PacketBufferOverflow => write!(f, "Packet too big for provided buffer!"),
            PacketParse(e) => write!(f, "Could not parse the packet into a valid command: {:?}", e),
            PacketUnexpected => write!(f, "Client sent an unexpected packet. Please re-run with `log` trace-level logging enabled and file an issue at https://github.com/daniel5151/gdbstub/issues"),
            TargetMismatch => write!(f, "GDB client sent a packet with too much data for the given target."),
            TargetError(e) => write!(f, "Target threw a fatal error: {:?}", e),
            UnsupportedStopReason => write!(f, "Target responded with an unsupported stop reason."),
            UnexpectedStepPacket => write!(f, "GDB client sent an unexpected `step` request. This is most-likely due to this GDB client bug: https://sourceware.org/bugzilla/show_bug.cgi?id=28440"),

            ImplicitSwBreakpoints => write!(f, "Warning: The target has not opted into using implicit software breakpoints. See `Target::guard_rail_implicit_sw_breakpoints` for more information."),
            MissingCurrentActivePidImpl => write!(f, "GDB client attempted to attach to a new process, but the target has not implemented support for `ExtendedMode::support_current_active_pid`"),

            NonFatalError(_) => write!(f, "Internal non-fatal error. End users should never see this! Please file an issue if you do!"),
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
