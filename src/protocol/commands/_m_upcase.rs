use super::prelude::*;

#[derive(PartialEq, Eq, Debug)]
pub struct M<'a> {
    pub addr: u64,
    pub len: usize,
    pub val: &'a [u8],
}

impl<'a> ParseCommand<'a> for M<'a> {
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        let body = buf.into_body();

        let mut body = body.split_mut(|&b| b == b',' || b == b':');
        let addr = decode_hex(body.next()?).ok()?;
        let len = decode_hex(body.next()?).ok()?;
        let val = body.next()?;

        Some(M {
            addr,
            len,
            val: decode_hex_buf(val).ok()?,
        })
    }
}
