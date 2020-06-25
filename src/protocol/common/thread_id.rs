use core::convert::TryFrom;
use core::str::FromStr;

#[derive(PartialEq, Eq, Debug)]
pub enum TidKind {
    All,
    Any,
    WithID(usize),
}

#[derive(PartialEq, Eq, Debug)]
pub struct Tid {
    pid: Option<TidKind>,
    tid: TidKind,
}

impl TryFrom<&str> for Tid {
    type Error = ();

    fn try_from(s: &str) -> Result<Self, ()> {
        if s.starts_with('p') {
            // p<pid>.<tid>
            let mut s = s.trim_start_matches('p').split('.');
            let pid = s.next().ok_or(())?.parse::<TidKind>().map_err(drop)?;
            let tid = s.next().ok_or(())?.parse::<TidKind>().map_err(drop)?;

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
    type Err = core::num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "-1" => TidKind::All,
            "0" => TidKind::Any,
            id => TidKind::WithID(usize::from_str_radix(id, 16)?),
        })
    }
}
