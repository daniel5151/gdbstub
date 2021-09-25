use super::prelude::*;

#[derive(Debug)]
pub struct X<'a> {
    pub addr: &'a [u8],
    pub len: usize,
    pub val: &'a [u8],
}

impl<'a> ParseCommand<'a> for X<'a> {
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        let body = buf.into_body();

        let mut body = body.split_mut_no_panic(|&b| b == b',' || b == b':');
        let addr = decode_hex_buf(body.next()?).ok()?;
        let len = decode_hex(body.next()?).ok()?;
        let val = decode_bin_buf(body.next()?).ok()?;

        Some(X { addr, len, val })
    }
}
