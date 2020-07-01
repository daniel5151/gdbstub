use super::prelude::*;

#[derive(PartialEq, Eq, Debug)]
pub struct g;

impl<'a> ParseCommand<'a> for g {
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        let body = buf.into_body_str();
        if !body.is_empty() {
            return None;
        }
        Some(g)
    }
}
