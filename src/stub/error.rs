use core::fmt::{self, Debug, Display};

use crate::arch::SingleStepGdbBehavior;
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
    /// Target didn't report any active threads when there should have been at
    /// least one running.
    NoActiveThreads,

    /// The target has not opted into using implicit software breakpoints.
    /// See [`Target::guard_rail_implicit_sw_breakpoints`] for more information.
    ///
    /// [`Target::guard_rail_implicit_sw_breakpoints`]:
    /// crate::target::Target::guard_rail_implicit_sw_breakpoints
    ImplicitSwBreakpoints,
    /// The target has not indicated support for optional single stepping. See
    /// [`Target::guard_rail_single_step_gdb_behavior`] for more information.
    ///
    /// If you encountered this error while using an `Arch` implementation
    /// defined in `gdbstub_arch` and believe this is incorrect, please file an
    /// issue at <https://github.com/daniel5151/gdbstub/issues>.
    ///
    /// [`Target::guard_rail_single_step_gdb_behavior`]:
    /// crate::target::Target::guard_rail_single_step_gdb_behavior
    SingleStepGdbBehavior(SingleStepGdbBehavior),

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
            NoActiveThreads => write!(f, "Target didn't report any active threads when there should have been at least one running."),

            ImplicitSwBreakpoints => write!(f, "Warning: The target has not opted into using implicit software breakpoints. See `Target::guard_rail_implicit_sw_breakpoints` for more information."),
            SingleStepGdbBehavior(behavior) => {
                use crate::arch::SingleStepGdbBehavior;
                write!(
                    f,
                    "Warning: Mismatch between the targets' single-step support and arch-level single-step behavior: {} ",
                    match behavior {
                        SingleStepGdbBehavior::Optional => "", // unreachable, since optional single step will not result in an error
                        SingleStepGdbBehavior::Required => "GDB requires single-step support on this arch.",
                        SingleStepGdbBehavior::Ignored => "GDB ignores single-step support on this arch, yet the target has implemented support for it.",
                        SingleStepGdbBehavior::Unknown => "This arch's single-step behavior hasn't been tested yet: please conduct a test + upstream your findings!",
                    }
                )?;
                write!(f, "See `Target::guard_rail_single_step_gdb_behavior` for more information.")
            },

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
