use super::prelude::*;

#[derive(Debug)]
pub struct G<'a> {
    pub vals: &'a [u8],
}

impl<'a> ParseCommand<'a> for G<'a> {
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        Some(G {
            vals: decode_hex_buf(buf.into_body()).ok()?,
        })
    }
}
