use super::prelude::*;

#[derive(PartialEq, Eq, Debug)]
pub struct T {
    pub tid: Tid,
}

impl<'a> ParseCommand<'a> for T {
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        let body = buf.into_body_str();
        let tid = body.parse::<Tid>().ok()?;
        Some(T { tid })
    }
}
