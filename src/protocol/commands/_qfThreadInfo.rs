use super::prelude::*;

#[derive(Debug)]
pub struct qfThreadInfo;

impl<'a> ParseCommand<'a> for qfThreadInfo {
    #[inline(always)]
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        if !buf.into_body().is_empty() {
            return None;
        }
        Some(qfThreadInfo)
    }
}
