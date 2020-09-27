use super::prelude::*;

#[derive(Debug)]
pub struct m<'a> {
    pub addr: u64,
    pub len: usize,

    pub buf: &'a mut [u8],
}

impl<'a> ParseCommand<'a> for m<'a> {
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        let body = buf.as_body();
        let mut body = body.split(|b| *b == b',');
        let addr = decode_hex(body.next()?).ok()?;
        let len = decode_hex(body.next()?).ok()?;

        Some(m {
            addr,
            len,
            buf: buf.into_raw_buf().0,
        })
    }
}
