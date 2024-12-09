use super::prelude::*;
use crate::target::ext::tracepoints::Tracepoint;

#[derive(Debug)]
pub struct qTP<'a> {
    pub tracepoint: Tracepoint,
    pub addr: &'a [u8],
}

impl<'a> ParseCommand<'a> for qTP<'a> {
    #[inline(always)]
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        let body = buf.into_body();
        match body {
            [b':', body @ ..] => {
                let mut s = body.split_mut(|b| *b == b':');
                let tracepoint = Tracepoint(decode_hex(s.next()?).ok()?);
                let addr = decode_hex_buf(s.next()?).ok()?;
                Some(qTP {
                    tracepoint,
                    addr
                })
            },
            _ => None,
        }
    }
}
