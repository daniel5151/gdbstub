use super::prelude::*;

#[derive(Debug)]
pub struct qXferMemoryMapRead<'a> {
    pub offset: u64,
    pub length: usize,

    pub buf: &'a mut [u8],
}

impl<'a> ParseCommand<'a> for qXferMemoryMapRead<'a> {
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        let (buf, body_range) = buf.into_raw_buf();
        let body = buf.get_mut(body_range.start..body_range.end)?;

        if body.is_empty() {
            return None;
        }

        let mut body = body.split(|b| *b == b':').skip(1);
        let annex = body.next()?;
        if annex != b"" {
            return None;
        }

        let mut body = body.next()?.split(|b| *b == b',');
        let offset = decode_hex(body.next()?).ok()?;
        let length = decode_hex(body.next()?).ok()?;

        drop(body);

        Some(qXferMemoryMapRead { offset, length , buf})
    }
}
