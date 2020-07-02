use super::prelude::*;

#[derive(PartialEq, Eq, Debug)]
pub struct QuestionMark;

impl<'a> ParseCommand<'a> for QuestionMark {
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        if !buf.into_body().is_empty() {
            return None;
        }
        Some(QuestionMark)
    }
}
