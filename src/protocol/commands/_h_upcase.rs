use super::prelude::*;

use crate::protocol::common::thread_id::ThreadId;

#[derive(Debug)]
pub enum Op {
    StepContinue,
    Other,
}

#[derive(Debug)]
pub struct H {
    pub kind: Op,
    pub thread: ThreadId,
}

impl<'a> ParseCommand<'a> for H {
    #[inline(always)]
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        let body = buf.into_body();
        if body.is_empty() {
            return None;
        }

        let kind = match body[0] {
            b'g' => Op::Other,
            b'c' => Op::StepContinue,
            _ => return None,
        };
        let thread: ThreadId = body[1..].try_into().ok()?;

        Some(H { kind, thread })
    }
}
