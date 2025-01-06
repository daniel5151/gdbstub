use super::prelude::*;

#[derive(Debug)]
pub struct qTBuffer<'a> {
    pub offset: u64,
    pub length: usize,
    pub data: &'a mut [u8]
}

impl<'a> ParseCommand<'a> for qTBuffer<'a> {
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
                // Our response has to be a hex encoded buffer that fits within
                // our packet size, which means we actually have half as much space
                // as our slice would indicate.
                let (front, _back) = buf.split_at_mut(buf.len() / 2);
                Some(qTBuffer { offset, length, data: front })
            },
            _ => None
        }
    }
}
