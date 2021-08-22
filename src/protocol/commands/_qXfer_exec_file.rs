use super::prelude::*;

use crate::common::Pid;

#[derive(Debug)]
pub struct qXferExecFileRead {
    pub pid: Option<Pid>,
    pub offset: usize,
    pub len: usize,
}

impl<'a> ParseCommand<'a> for qXferExecFileRead {
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        let body = buf.into_body();

        if body.is_empty() {
            return None;
        }

        let mut body = body.split(|b| *b == b':').skip(1);
        let pid = decode_hex(body.next()?).ok().and_then(Pid::new);

        let mut body = body.next()?.split(|b| *b == b',');
        let offset = decode_hex(body.next()?).ok()?;
        let len = decode_hex(body.next()?).ok()?;

        Some(qXferExecFileRead {pid, offset, len})
    }
}
