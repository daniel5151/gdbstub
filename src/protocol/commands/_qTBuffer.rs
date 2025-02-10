use super::prelude::*;

#[derive(Debug)]
pub struct qTBuffer {
    pub offset: u64,
    pub length: usize,
}

impl<'a> ParseCommand<'a> for qTBuffer {
    #[inline(always)]
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        let (buf, body_range) = buf.into_raw_buf();
        let body = &buf[body_range];
        match body {
            [b':', body @ ..] => {
                let mut req_opts = body.split(|b| *b == b',');
                let (offset, length) = (req_opts.next()?, req_opts.next()?);
                let offset = decode_hex(offset).ok()?;
                let length = decode_hex(length).ok()?;
                Some(qTBuffer { offset, length })
            }
            _ => None,
        }
    }
}
