use crate::stub::GdbStubError;
use crate::target::TargetError;

/// Extension trait to ease working with `TargetResult` in the GdbStub
/// implementation.
pub(super) trait TargetResultExt<V, T, C> {
    /// Encapsulates the boilerplate associated with handling `TargetError`s,
    /// such as bailing-out on Fatal errors, or returning response codes.
    fn handle_error(self) -> Result<V, GdbStubError<T, C>>;
}

impl<V, T, C> TargetResultExt<V, T, C> for Result<V, TargetError<T>> {
    fn handle_error(self) -> Result<V, GdbStubError<T, C>> {
        let code = match self {
            Ok(v) => return Ok(v),
            Err(TargetError::Fatal(e)) => return Err(GdbStubError::TargetError(e)),
            // Recoverable errors:
            // Error code 121 corresponds to `EREMOTEIO` lol
            Err(TargetError::NonFatal) => 121,
            Err(TargetError::Errno(code)) => code,
            #[cfg(feature = "std")]
            Err(TargetError::Io(e)) => e.raw_os_error().unwrap_or(121) as u8,
        };

        Err(GdbStubError::NonFatalError(code))
    }
}
