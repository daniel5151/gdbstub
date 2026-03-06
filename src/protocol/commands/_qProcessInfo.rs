use super::prelude::*;

#[derive(Debug)]
pub struct qProcessInfo;

impl<'a> ParseCommand<'a> for qProcessInfo {
    #[inline(always)]
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        if !buf.into_body().is_empty() {
            return None;
        }
        Some(qProcessInfo)
    }
}
