use super::prelude::*;

#[derive(PartialEq, Eq, Debug)]
pub struct s {
    pub addr: Option<u64>,
}

impl<'a> ParseCommand<'a> for s {
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        let body = buf.into_body();
        if body.is_empty() {
            return Some(s { addr: None });
        }

        let addr = decode_hex(&body[1..]).ok()?;
        Some(s { addr: Some(addr) })
    }
}
