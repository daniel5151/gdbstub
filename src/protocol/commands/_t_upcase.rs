use super::prelude::*;
use crate::protocol::common::thread_id::ThreadId;

#[derive(Debug)]
pub struct T {
    pub thread: ThreadId,
}

impl<'a> ParseCommand<'a> for T {
    #[inline(always)]
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        let body = buf.into_body();
        Some(T {
            thread: body.try_into().ok()?,
        })
    }
}
