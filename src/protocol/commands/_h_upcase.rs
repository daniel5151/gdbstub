use super::prelude::*;

#[derive(PartialEq, Eq, Debug)]
pub struct H {
    pub kind: char, // TODO: make this an enum
    pub tid: Tid,
}

impl<'a> ParseCommand<'a> for H {
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        let body = buf.into_body_str();
        if body.is_empty() {
            return None;
        }

        let kind = body.chars().next()?;
        let tid = body[1..].parse::<Tid>().ok()?;

        Some(H { kind, tid })
    }
}
