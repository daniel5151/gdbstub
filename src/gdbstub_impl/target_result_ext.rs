use crate::protocol::ResponseWriter;
use crate::target::TargetError;
use crate::Connection;
use crate::GdbStubError;

/// Extension trait to ease working with `TargetResult` in the GdbStub
/// implementation.
pub trait TargetResultExt<V, T, C: Connection> {
    /// Encapsulates the boilerplate associated with handling `TargetError`s,
    /// such as bailing-out on Fatal errors, or returning response codes.
    fn handle_error(self, res: &mut ResponseWriter<C>) -> Result<V, GdbStubError<T, C::Error>>;
}

impl<V, T, C: Connection> TargetResultExt<V, T, C> for Result<V, TargetError<T>> {
    fn handle_error(self, res: &mut ResponseWriter<C>) -> Result<V, GdbStubError<T, C::Error>> {
        let code = match self {
            Ok(v) => return Ok(v),
            Err(TargetError::Fatal(e)) => return Err(GdbStubError::TargetError(e)),
            Err(TargetError::Errno(code)) => code,
            // Error code 121 corresponds to `EREMOTEIO` :D
            #[cfg(feature = "std")]
            Err(TargetError::Io(e)) => e.raw_os_error().unwrap_or(121) as u8,
        };

        res.write_str("E")?;
        res.write_num(code)?;
        Err(GdbStubError::SendErrorCode)
    }
}
