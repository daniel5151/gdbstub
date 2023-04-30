use super::prelude::*;
use crate::protocol::common::thread_id::ThreadId;
use crate::protocol::ConcreteThreadId;

#[derive(Debug)]
pub struct qThreadExtraInfo<'a> {
    pub id: ConcreteThreadId,

    pub buf: &'a mut [u8],
}

impl<'a> ParseCommand<'a> for qThreadExtraInfo<'a> {
    #[inline(always)]
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        let (buf, body_range) = buf.into_raw_buf();
        let body = buf.get(body_range.start..body_range.end)?;

        if body.is_empty() {
            return None;
        }

        match body {
            [b',', body @ ..] => {
                let id = ConcreteThreadId::try_from(ThreadId::try_from(body).ok()?).ok()?;

                Some(qThreadExtraInfo { id, buf })
            }
            _ => None,
        }
    }
}
