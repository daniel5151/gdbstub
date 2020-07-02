use super::prelude::*;

#[derive(PartialEq, Eq, Debug)]
pub struct m {
    pub addr: u64,
    pub len: usize,
}

impl<'a> ParseCommand<'a> for m {
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        let body = buf.into_body_str();
        let mut body = body.split(',');
        let addr = decode_hex(body.next()?.as_ref()).ok()?;
        let len = decode_hex(body.next()?.as_ref()).ok()?;

        Some(m { addr, len })
    }
}
