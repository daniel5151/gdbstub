use super::prelude::*;

#[derive(Debug)]
pub struct g;

impl<'a> ParseCommand<'a> for g {
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        if !buf.into_body().is_empty() {
            return None;
        }
        Some(g)
    }
}
