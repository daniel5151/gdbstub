use super::prelude::*;

#[derive(Debug)]
pub struct c<'a> {
    pub addr: Option<&'a [u8]>,
}

impl<'a> ParseCommand<'a> for c<'a> {
    #[inline(always)]
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        let body = buf.into_body();
        if body.is_empty() {
            return Some(c { addr: None });
        }
        let addr = match body {
            [] => None,
            _ => Some(decode_hex_buf(body).ok()? as &[u8]),
        };
        Some(c { addr })
    }
}
