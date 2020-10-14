use super::prelude::*;

#[derive(Debug)]
pub struct z<'a> {
    pub type_: u8,
    pub addr: &'a [u8],
    pub kind: u8,
}

impl<'a> ParseCommand<'a> for z<'a> {
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        let body = buf.into_body();
        let mut body = body.split_mut(|&b| b == b',');
        let type_ = decode_hex(body.next()?).ok()?;
        let addr = decode_hex_buf(body.next()?).ok()?;
        let kind = decode_hex(body.next()?).ok()?;

        Some(z { type_, addr, kind })
    }
}
