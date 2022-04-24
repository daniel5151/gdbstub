use super::prelude::*;

#[derive(Debug)]
pub struct QuestionMark;

impl<'a> ParseCommand<'a> for QuestionMark {
    #[inline(always)]
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        if !buf.into_body().is_empty() {
            return None;
        }
        Some(QuestionMark)
    }
}
