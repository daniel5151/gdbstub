use core::convert::{TryFrom, TryInto};
use core::num::NonZeroUsize;

use super::decode_hex;

/// Tid/Pid Selector.
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum IdKind {
    /// All threads (-1)
    All,
    /// Any thread (0)
    Any,
    /// Thread with specific ID (id > 0)
    WithID(NonZeroUsize),
}

/// Unique Thread ID.
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub struct ThreadId {
    /// Process ID (may or may not be present).
    pub pid: Option<IdKind>,
    /// Thread ID.
    pub tid: IdKind,
}

impl TryFrom<&[u8]> for ThreadId {
    type Error = ();

    fn try_from(s: &[u8]) -> Result<Self, ()> {
        match s {
            [b'p', s @ ..] => {
                // p<pid>.<tid>
                let mut s = s.split(|b| *b == b'.');
                let pid: IdKind = s.next().ok_or(())?.try_into()?;
                let tid: IdKind = match s.next() {
                    Some(s) => s.try_into()?,
                    None => IdKind::All, // sending only p<pid> is valid
                };

                Ok(ThreadId {
                    pid: Some(pid),
                    tid,
                })
            }
            _ => {
                // <tid>
                let tid: IdKind = s.try_into()?;

                Ok(ThreadId { pid: None, tid })
            }
        }
    }
}

impl TryFrom<&[u8]> for IdKind {
    type Error = ();

    fn try_from(s: &[u8]) -> Result<Self, ()> {
        Ok(match s {
            b"-1" => IdKind::All,
            b"0" => IdKind::Any,
            id => IdKind::WithID(NonZeroUsize::new(decode_hex(id).map_err(drop)?).ok_or(())?),
        })
    }
}

impl TryFrom<&mut [u8]> for ThreadId {
    type Error = ();

    fn try_from(s: &mut [u8]) -> Result<Self, ()> {
        Self::try_from(s as &[u8])
    }
}

impl TryFrom<&mut [u8]> for IdKind {
    type Error = ();

    fn try_from(s: &mut [u8]) -> Result<Self, ()> {
        Self::try_from(s as &[u8])
    }
}
