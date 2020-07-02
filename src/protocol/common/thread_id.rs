use core::convert::TryFrom;
use core::str::FromStr;

use super::decode_hex;

/// Thread Identifier.
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum TidKind {
    /// All threads
    All,
    /// Any thread
    Any,
    /// Thread with specific ID
    WithID(usize),
}

/// Thread ID
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub struct Tid {
    pub pid: Option<TidKind>,
    pub tid: TidKind,
}

impl TryFrom<&str> for Tid {
    type Error = ();

    fn try_from(s: &str) -> Result<Self, ()> {
        if s.starts_with('p') {
            // p<pid>.<tid>
            let mut s = s.trim_start_matches('p').split('.');
            let pid = s.next().ok_or(())?.parse::<TidKind>().map_err(drop)?;
            let tid = match s.next() {
                Some(s) => s.parse::<TidKind>().map_err(drop)?,
                None => TidKind::All, // valid to pass only p<pid>
            };

            Ok(Tid {
                pid: Some(pid),
                tid,
            })
        } else {
            // <tid>
            let tid = s.parse::<TidKind>().map_err(drop)?;

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

impl FromStr for TidKind {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "-1" => TidKind::All,
            "0" => TidKind::Any,
            id => TidKind::WithID(decode_hex(id.as_bytes()).map_err(drop)?),
        })
    }
}
