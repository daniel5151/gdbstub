use core::convert::TryFrom;

use crate::protocol::common::TidKind;

#[derive(PartialEq, Eq, Debug)]
pub struct vKill {
    pub pid: TidKind,
}

impl TryFrom<&str> for vKill {
    type Error = ();

    fn try_from(body: &str) -> Result<Self, ()> {
        let pid = body
            .trim_start_matches(';')
            .parse::<TidKind>()
            .map_err(drop)?;
        Ok(vKill { pid })
    }
}
