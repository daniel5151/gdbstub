use core::convert::TryFrom;
use core::num::NonZeroUsize;
use core::str::FromStr;

use super::decode_hex;

/// Thread ID Selector.
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum TidSelector {
    /// All threads (-1)
    All,
    /// Any thread (0)
    Any,
    /// Thread with specific ID (id > 0)
    WithID(NonZeroUsize),
}

/// Thread ID.
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub struct Tid {
    /// Process ID (may or may not be present).
    pub pid: Option<TidSelector>,
    /// Thread ID.
    pub tid: TidSelector,
}

impl TryFrom<&str> for Tid {
    type Error = ();

    fn try_from(s: &str) -> Result<Self, ()> {
        if s.starts_with('p') {
            // p<pid>.<tid>
            let mut s = s.trim_start_matches('p').split('.');
            let pid = s.next().ok_or(())?.parse::<TidSelector>().map_err(drop)?;
            let tid = match s.next() {
                Some(s) => s.parse::<TidSelector>().map_err(drop)?,
                None => TidSelector::All, // valid to pass only p<pid>
            };

            Ok(Tid {
                pid: Some(pid),
                tid,
            })
        } else {
            // <tid>
            let tid = s.parse::<TidSelector>().map_err(drop)?;

            Ok(Tid { pid: None, tid })
        }
    }
}

impl FromStr for Tid {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, ()> {
        Tid::try_from(s)
    }
}

impl FromStr for TidSelector {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "-1" => TidSelector::All,
            "0" => TidSelector::Any,
            id => TidSelector::WithID(
                NonZeroUsize::new(decode_hex(id.as_bytes()).map_err(drop)?).ok_or(())?,
            ),
        })
    }
}
