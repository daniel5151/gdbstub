use core::convert::TryFrom;

use crate::protocol::common::TidKind;

#[derive(PartialEq, Eq, Debug)]
pub struct D {
    pub pid: Option<TidKind>,
}

impl TryFrom<&str> for D {
    type Error = ();

    fn try_from(body: &str) -> Result<Self, ()> {
        let pid = body.trim_start_matches(';').parse::<TidKind>().ok();
        Ok(D { pid })
    }
}
