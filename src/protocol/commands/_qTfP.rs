use super::prelude::*;

#[derive(Debug)]
pub struct qTfP;

impl<'a> ParseCommand<'a> for qTfP {
    #[inline(always)]
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        if !buf.into_body().is_empty() {
            return None;
        }
        Some(qTfP)
    }
}
