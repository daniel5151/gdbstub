use crate::protocol::PacketParseError;
use crate::protocol::ResponseWriterError;
use crate::util::managed_vec::CapacityError;
#[cfg(feature = "core_error")]
use core::error::Error as CoreError;
use core::fmt::Debug;
use core::fmt::Display;
use core::fmt::{self};
#[cfg(all(feature = "std", not(feature = "core_error")))]
use std::error::Error as CoreError;

/// An error that may occur while interacting with a
/// [`Connection`](crate::conn::Connection).
#[derive(Debug)]
pub enum ConnectionErrorKind {
    /// Error initializing the session.
    Init,
    /// Error reading data.
    Read,
    /// Error writing data.
    Write,
}

#[derive(Debug)]
pub(crate) enum InternalError<T, C> {
    /// Connection Error
    Connection(C, ConnectionErrorKind),
    /// Target encountered a fatal error.
    TargetError(T),

    ClientSentNack,
    PacketBufferOverflow,
    PacketParse(PacketParseError),
    PacketUnexpected,
    TargetMismatch,
    UnsupportedStopReason,
    UnexpectedStepPacket,
    ImplicitSwBreakpoints,
    // DEVNOTE: this is a temporary workaround for something that can and should
    // be caught at compile time via IDETs. That said, since i'm not sure when
    // I'll find the time to cut a breaking release of gdbstub, I'd prefer to
    // push out this feature as a non-breaking change now.
    MissingCurrentActivePidImpl,
    TracepointFeatureUnimplemented(u8),
    TracepointUnsupportedSourceEnumeration,
    MissingMultiThreadSchedulerLocking,

    // Internal - A non-fatal error occurred (with errno-style error code)
    //
    // This "dummy" error is required as part of the internal
    // `TargetResultExt::handle_error()` machinery, and will never be
    // propagated up to the end user.
    #[doc(hidden)]
    NonFatalError(u8),
}

impl<T, C> InternalError<T, C> {
    pub fn conn_read(e: C) -> Self {
        InternalError::Connection(e, ConnectionErrorKind::Read)
    }

    pub fn conn_write(e: C) -> Self {
        InternalError::Connection(e, ConnectionErrorKind::Write)
    }

    pub fn conn_init(e: C) -> Self {
        InternalError::Connection(e, ConnectionErrorKind::Init)
    }
}

impl<T, C> From<ResponseWriterError<C>> for InternalError<T, C> {
    fn from(e: ResponseWriterError<C>) -> Self {
        InternalError::Connection(e.0, ConnectionErrorKind::Write)
    }
}

// these macros are used to keep the docs and `Display` impl in-sync

macro_rules! unsupported_stop_reason {
    () => {
        "User error: cannot report stop reason without also implementing its corresponding IDET"
    };
}

macro_rules! unexpected_step_packet {
    () => {
        "Received an unexpected `step` request. This is most-likely due to this GDB client bug: <https://sourceware.org/bugzilla/show_bug.cgi?id=28440>"
    };
}

/// An error which may occur during a GDB debugging session.
///
/// ## Additional Notes
///
/// `GdbStubError`'s inherent `Display` impl typically contains enough context
/// for users to understand why the error occurred.
///
/// That said, there are a few instances where the error condition requires
/// additional context.
///
/// * * *
#[doc = concat!("_", unsupported_stop_reason!(), "_")]
///
/// This is a not a bug with `gdbstub`. Rather, this is indicative of a bug in
/// your `gdbstub` integration.
///
/// Certain stop reasons can only be used when their associated protocol feature
/// has been implemented. e.g: a Target cannot return a `StopReason::HwBreak` if
/// the hardware breakpoints IDET hasn't been implemented.
///
/// Please double-check that you've implemented all the necessary `supports_`
/// methods related to the stop reason you're trying to report.
///
/// * * *
#[doc = concat!("_", unexpected_step_packet!(), "_")]
///
/// Unfortunately, there's nothing `gdbstub` can do to work around this bug.
///
/// Until the issue is fixed upstream, certain architectures are essentially
/// forced to manually implement single-step support.
#[derive(Debug)]
pub struct GdbStubError<T, C> {
    kind: InternalError<T, C>,
}

impl<T, C> Display for GdbStubError<T, C>
where
    C: Display,
    T: Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use self::InternalError::*;
        const CONTEXT: &str = "See the `GdbStubError` docs for more details";
        match &self.kind {
            Connection(e, ConnectionErrorKind::Init) => write!(f, "Connection Error while initializing the session: {}", e),
            Connection(e, ConnectionErrorKind::Read) => write!(f, "Connection Error while reading request: {}", e),
            Connection(e, ConnectionErrorKind::Write) => write!(f, "Connection Error while writing response: {}", e),
            ClientSentNack => write!(f, "Client nack'd the last packet, but `gdbstub` doesn't implement re-transmission."),
            PacketBufferOverflow => write!(f, "Received an oversized packet (did not fit in provided packet buffer)"),
            PacketParse(e) => write!(f, "Failed to parse packet into a valid command: {:?}", e),
            PacketUnexpected => write!(f, "Client sent an unexpected packet. This should never happen! Please re-run with `log` trace-level logging enabled and file an issue at https://github.com/daniel5151/gdbstub/issues"),
            TargetMismatch => write!(f, "Received a packet with too much data for the given target"),
            TargetError(e) => write!(f, "Target threw a fatal error: {}", e),
            UnsupportedStopReason => write!(f, "{} {}", unsupported_stop_reason!(), CONTEXT),
            UnexpectedStepPacket => write!(f, "{} {}", unexpected_step_packet!(), CONTEXT),

            ImplicitSwBreakpoints => write!(f, "Warning: The target has not opted into using implicit software breakpoints. See `Target::guard_rail_implicit_sw_breakpoints` for more information"),
            MissingCurrentActivePidImpl => write!(f, "GDB client attempted to attach to a new process, but the target has not implemented support for `ExtendedMode::support_current_active_pid`"),
            TracepointFeatureUnimplemented(feat) => write!(f, "GDB client sent us a tracepoint packet using feature {}, but `gdbstub` doesn't implement it. If this is something you require, please file an issue at https://github.com/daniel5151/gdbstub/issues", *feat as char),
            TracepointUnsupportedSourceEnumeration => write!(f, "The target doesn't support the gdbstub TracepointSource extension, but attempted to transition to enumerating tracepoint sources"),
            MissingMultiThreadSchedulerLocking => write!(f, "GDB requested Scheduler Locking, but the Target does not implement the `MultiThreadSchedulerLocking` IDET"),

            NonFatalError(_) => write!(f, "Internal non-fatal error. You should never see this! Please file an issue if you do!"),
        }
    }
}

#[cfg(any(feature = "std", feature = "core_error"))]
impl<T, C> CoreError for GdbStubError<T, C>
where
    C: Debug + Display,
    T: Debug + Display,
{
}

impl<T, C> GdbStubError<T, C> {
    /// Check if the error was due to a target error.
    pub fn is_target_error(&self) -> bool {
        matches!(self.kind, InternalError::TargetError(..))
    }

    /// If the error was due to a target error, return the concrete error type.
    pub fn into_target_error(self) -> Option<T> {
        match self.kind {
            InternalError::TargetError(e) => Some(e),
            _ => None,
        }
    }

    /// Check if the error was due to a connection error.
    pub fn is_connection_error(&self) -> bool {
        matches!(self.kind, InternalError::Connection(..))
    }

    /// If the error was due to a connection error, return the concrete error
    /// type.
    pub fn into_connection_error(self) -> Option<(C, ConnectionErrorKind)> {
        match self.kind {
            InternalError::Connection(e, kind) => Some((e, kind)),
            _ => None,
        }
    }
}

impl<T, C> From<InternalError<T, C>> for GdbStubError<T, C> {
    fn from(kind: InternalError<T, C>) -> Self {
        GdbStubError { kind }
    }
}

impl<A, T, C> From<CapacityError<A>> for GdbStubError<T, C> {
    fn from(_: CapacityError<A>) -> Self {
        InternalError::PacketBufferOverflow.into()
    }
}
