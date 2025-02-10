use super::prelude::*;
use crate::target::ext::tracepoints::Tracepoint;
use crate::target::ext::tracepoints::TracepointSourceType;

pub struct QTDPsrc<'a> {
    pub number: Tracepoint,
    pub addr: &'a [u8],
    pub kind: TracepointSourceType,
    pub start: u32,
    pub slen: u32,
    pub bytes: &'a mut [u8],
}

impl<'a> ParseCommand<'a> for QTDPsrc<'a> {
    #[inline(always)]
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        let body = buf.into_body();
        match body {
            [b':', info @ ..] => {
                let mut params = info.splitn_mut(7, |b| *b == b':');
                let number = Tracepoint(decode_hex(params.next()?).ok()?);
                let addr = decode_hex_buf(params.next()?).ok()?;
                let kind = match params.next()?.as_ref() {
                    b"at" => Some(TracepointSourceType::At),
                    b"cond" => Some(TracepointSourceType::Cond),
                    b"cmd" => Some(TracepointSourceType::Cmd),
                    _ => None,
                }?;
                let start = decode_hex(params.next()?).ok()?;
                let slen = decode_hex(params.next()?).ok()?;
                let bytes = decode_hex_buf(params.next()?).ok()?;
                Some(QTDPsrc {
                    number,
                    addr,
                    kind,
                    start,
                    slen,
                    bytes,
                })
            }
            _ => None,
        }
    }
}
