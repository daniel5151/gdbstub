use super::prelude::*;

#[derive(Debug)]
pub struct qsThreadInfo;

impl<'a> ParseCommand<'a> for qsThreadInfo {
    #[inline(always)]
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        if !buf.into_body().is_empty() {
            return None;
        }
        Some(qsThreadInfo)
    }
}
