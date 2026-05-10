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
    Connection(C, ConnectionErrorKind),
    TargetError(T),

    // Errors indicating a GDB client issue, out of `gdbstub`'s control
    ClientSentNack,
    PacketBufferOverflow,
    PacketParse(PacketParseError),
    PacketUnexpected,
    TracepointFeatureUnimplemented(u8),
    UnexpectedIntegerSize,
    UnexpectedReg,
    UnexpectedStepPacket,
    UnexpectedThreadId,

    // Errors indicative of a error in the user's `Target` implementation / `gdbstub` integration.
    ImplicitSwBreakpoints,
    MissingToRawId,
    TracepointUnsupportedSourceEnumeration,
    UnsupportedStopReason,

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

    fn prefix(&self) -> &'static str {
        use self::InternalError::*;

        match self {
            Connection(_, _) => "Connection",

            ClientSentNack
            | PacketBufferOverflow
            | PacketParse(_)
            | PacketUnexpected
            | TracepointFeatureUnimplemented(_)
            | UnexpectedIntegerSize
            | UnexpectedReg
            | UnexpectedStepPacket
            | UnexpectedThreadId => "Client",

            TargetError(_)
            | ImplicitSwBreakpoints
            | MissingToRawId
            | TracepointUnsupportedSourceEnumeration
            | UnsupportedStopReason => "Target",

            NonFatalError(_) => "Unreachable",
        }
    }

    fn should_file_bug_report(&self) -> bool {
        use self::InternalError::*;

        match self {
            ClientSentNack
            | Connection(_, _)
            | ImplicitSwBreakpoints
            | MissingToRawId
            | PacketBufferOverflow
            | PacketParse(_)
            | TargetError(_)
            | TracepointFeatureUnimplemented(_)
            | TracepointUnsupportedSourceEnumeration
            | UnexpectedStepPacket
            | UnsupportedStopReason => false,

            UnexpectedIntegerSize | PacketUnexpected | UnexpectedReg | UnexpectedThreadId => true,

            NonFatalError(_) => true,
        }
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
        "User error: cannot report stop reason without also activating its corresponding IDET"
    };
}

macro_rules! unexpected_step_packet {
    () => {
        "Sent us an unexpected `step` request. This is most-likely due to this GDB client bug: <https://sourceware.org/bugzilla/show_bug.cgi?id=28440>"
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
/// has been enabled. For example:
///
/// - e.g: To use `StopReasonReporter::hwbreak`, `supports_hw_breakpoints` must
///   return `Some`
/// - e.g: To use `StopReasonReporter::vfork`, `use_vfork_stop_reason` must
///   return `true`.
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

        const SEE_DOCS_FOR_CONTEXT: &str = "See the `GdbStubError` docs for more details";

        write!(f, "{} error: ", self.kind.prefix())?;

        match &self.kind {
            Connection(e, ConnectionErrorKind::Init) => write!(f, "error initializing the session: {e}"),
            Connection(e, ConnectionErrorKind::Read) => write!(f, "error while reading: {e}"),
            Connection(e, ConnectionErrorKind::Write) => write!(f, "error while writing: {e}"),
            TargetError(e) => write!(f, "Target threw a fatal error: {e}"),

            // Errors indicating a GDB client issue, out of `gdbstub`'s control
            ClientSentNack => write!(f, "Sent us a nack packet, but `gdbstub` doesn't implement re-transmission. See https://github.com/daniel5151/gdbstub/issues/137"),
            PacketBufferOverflow => write!(f, "Sent us an oversized packet (doesn't fit in the configured packet buffer)"),
            PacketParse(e) => write!(f, "Sent us packet that couldn't be parsed: {e:?}"),
            PacketUnexpected => write!(f, "Sent us a packet `gdbstub` wasn't expecting"),
            TracepointFeatureUnimplemented(feat) => write!(f, "Sent us a tracepoint packet using feature {}, but `gdbstub` doesn't implement it. If this is something you require, please file an issue at https://github.com/daniel5151/gdbstub/issues", *feat as char),
            UnexpectedIntegerSize => write!(f, "Sent us packet exceeding the integer bounds for the given target"),
            UnexpectedReg => write!(f, "Sent us a packet with register data that isn't compatible with the current Target"),
            UnexpectedStepPacket => write!(f, "{} {}", unexpected_step_packet!(), SEE_DOCS_FOR_CONTEXT),
            UnexpectedThreadId => write!(f, "Sent us a packet with an unexpected thread ID for the given target"),

            // Errors indicating an error in the user's `Target` implementation / `gdbstub` integration.
            ImplicitSwBreakpoints => write!(f, "The target has not opted into using implicit software breakpoints. See `Target::guard_rail_implicit_sw_breakpoints` for more information"),
            MissingToRawId => write!(f, "A RegId was used with an API that requires raw register IDs to be available (e.g. `StopReasonReporter::add_reg`) but returned `None` from `to_raw_id()`"),
            TracepointUnsupportedSourceEnumeration => write!(f, "The target doesn't support the gdbstub TracepointSource extension, but attempted to transition to enumerating tracepoint sources"),
            UnsupportedStopReason => write!(f, "{} {}", unsupported_stop_reason!(), SEE_DOCS_FOR_CONTEXT),

            NonFatalError(_) => write!(f, "Internal non-fatal error. You should never see this!"),
        }?;

        if self.kind.should_file_bug_report() {
            write!(f, ". This should never happen! Please re-run with `log` trace-level logging enabled and file an issue at https://github.com/daniel5151/gdbstub/issues")?;
        }

        Ok(())
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

    /// Create a new error wrapping a target error.
    pub fn from_target_error(err: T) -> Self {
        Self {
            kind: InternalError::TargetError(err),
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
