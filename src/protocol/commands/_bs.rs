use super::prelude::*;

#[derive(Debug)]
pub struct bs;

impl<'a> ParseCommand<'a> for bs {
    #[inline(always)]
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        if !buf.into_body().is_empty() {
            return None;
        }
        Some(bs)
    }
}
