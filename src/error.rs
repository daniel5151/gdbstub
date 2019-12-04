use core::fmt::{Debug, Display};

#[derive(Debug)]
pub enum Error<T, C> {
    Connection(C),
    TargetError(T),
    Unexpected,
    ChecksumParse,
    MismatchedChecksum,
    CommandParse(String),
}

impl<T: Debug, C: Debug> Display for Error<T, C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use self::Error::*;
        match self {
            Connection(e) => write!(f, "Connection Error: {:?}", e),
            TargetError(e) => write!(f, "Target Fatal Error: {:?}", e),
            Unexpected => write!(f, "Client sent an unexpected packet"),
            ChecksumParse => write!(f, "Couldn't parse checksum"),
            MismatchedChecksum => write!(f, "Checksum mismatch"),
            CommandParse(e) => write!(f, "Couldn't parse command: {}", e),
        }
    }
}

impl<T: Debug, C: Debug> std::error::Error for Error<T, C> {}
