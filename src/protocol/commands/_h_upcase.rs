use core::convert::TryFrom;

use crate::protocol::common::Tid;

#[derive(PartialEq, Eq, Debug)]
pub struct H {
    pub kind: char, // TODO: make this an enum
    pub tid: Tid,
}

impl TryFrom<&str> for H {
    type Error = ();

    fn try_from(body: &str) -> Result<Self, ()> {
        if body.is_empty() {
            return Err(());
        }

        let kind = body.chars().next().ok_or(())?;
        let tid = body[1..].parse::<Tid>().map_err(drop)?;

        Ok(H { kind, tid })
    }
}
