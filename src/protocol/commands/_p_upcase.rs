use super::prelude::*;

#[derive(PartialEq, Eq, Debug)]
pub struct P<'a> {
    pub reg_id: usize,
    pub val: &'a [u8]
}

impl<'a> ParseCommand<'a> for P<'a> {
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        let body = buf.into_body();
        let mut body = body.split_mut(|&b| b == b'=');
        let reg_id = decode_hex(body.next()?).ok()?;
        let val = decode_hex_buf(body.next()?).ok()?;
        Some(P { reg_id, val })
    }
}
