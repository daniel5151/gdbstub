use super::prelude::*;

#[derive(PartialEq, Eq, Debug)]
pub struct ExclamationMark;

impl<'a> ParseCommand<'a> for ExclamationMark {
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        if !buf.into_body().is_empty() {
            return None;
        }
        Some(ExclamationMark)
    }
}
