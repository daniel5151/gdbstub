use super::prelude::*;

#[derive(Debug)]
pub struct qTsP;

impl<'a> ParseCommand<'a> for qTsP {
    #[inline(always)]
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        if !buf.into_body().is_empty() {
            return None;
        }
        Some(qTsP)
    }
}
