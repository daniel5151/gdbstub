use super::prelude::*;

#[derive(PartialEq, Eq, Debug)]
pub struct c {
    pub addr: Option<u64>,
}

impl<'a> ParseCommand<'a> for c {
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        let body = buf.into_body();
        if body.is_empty() {
            return Some(c { addr: None });
        }
        let addr = decode_hex(body).ok()?;
        Some(c { addr: Some(addr) })
    }
}
