use super::prelude::*;
use crate::protocol::common::thread_id::ThreadId;

#[derive(Debug)]
pub struct qWasmCallStack {
    pub tid: ThreadId,
}

impl<'a> ParseCommand<'a> for qWasmCallStack {
    #[inline(always)]
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        let body = buf.into_body();
        if body.is_empty() || body[0] != b':' {
            return None;
        }
        let tid = &body[1..];
        let tid = ThreadId::try_from(tid).ok()?;
        Some(qWasmCallStack { tid })
    }
}
