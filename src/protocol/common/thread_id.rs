use core::convert::TryFrom;
use core::num::NonZeroUsize;
use core::str::FromStr;

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

impl TryFrom<&str> for ThreadId {
    type Error = ();

    fn try_from(s: &str) -> Result<Self, ()> {
        if s.starts_with('p') {
            // p<pid>.<tid>
            let mut s = s.trim_start_matches('p').split('.');
            let pid = s.next().ok_or(())?.parse::<IdKind>().map_err(drop)?;
            let tid = match s.next() {
                Some(s) => s.parse::<IdKind>().map_err(drop)?,
                None => IdKind::All, // sending only p<pid> is valid
            };

            Ok(ThreadId {
                pid: Some(pid),
                tid,
            })
        } else {
            // <tid>
            let tid = s.parse::<IdKind>().map_err(drop)?;

            Ok(ThreadId { pid: None, tid })
        }
    }
}

impl FromStr for ThreadId {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, ()> {
        ThreadId::try_from(s)
    }
}

impl FromStr for IdKind {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "-1" => IdKind::All,
            "0" => IdKind::Any,
            id => IdKind::WithID(
                NonZeroUsize::new(decode_hex(id.as_bytes()).map_err(drop)?).ok_or(())?,
            ),
        })
    }
}
