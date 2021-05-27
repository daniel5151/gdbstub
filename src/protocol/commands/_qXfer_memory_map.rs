use super::prelude::*;

#[derive(Debug)]
pub struct qXferMemoryMapRead {
    pub offset: usize,
    pub len: usize,
}

impl<'a> ParseCommand<'a> for qXferMemoryMapRead {
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        let body = buf.into_body();

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
        let len = decode_hex(body.next()?).ok()?;

        Some(qXferMemoryMapRead { offset, len })
    }
}
