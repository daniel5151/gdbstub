use super::prelude::*;

use crate::common::Pid;

#[derive(Debug)]
pub struct qXferExecFileRead<'a> {
    pub pid: Option<Pid>,
    pub offset: &'a [u8],
    pub length: &'a [u8],

    pub buf: &'a mut [u8],
}

impl<'a> ParseCommand<'a> for qXferExecFileRead<'a> {
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        let (buf, body_range) = buf.into_raw_buf();
        let (body, buf) = buf[body_range.start..].split_at_mut(body_range.end - body_range.start);

        if body.is_empty() {
            return None;
        }

        let mut body = body.split_mut_no_panic(|b| *b == b':').skip(1);
        let pid = decode_hex(body.next()?).ok().and_then(Pid::new);

        let mut body = body.next()?.split_mut_no_panic(|b| *b == b',');
        let offset = decode_hex_buf(body.next()?).ok()?;
        let length = decode_hex_buf(body.next()?).ok()?;

        drop(body);

        Some(qXferExecFileRead {pid, offset, length, buf})
    }
}
