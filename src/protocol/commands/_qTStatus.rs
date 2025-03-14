use super::prelude::*;

#[derive(Debug)]
pub struct qTStatus;

impl<'a> ParseCommand<'a> for qTStatus {
    #[inline(always)]
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        if !buf.into_body().is_empty() {
            return None;
        }
        Some(qTStatus)
    }
}
