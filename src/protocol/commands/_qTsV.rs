use super::prelude::*;

#[derive(Debug)]
pub struct qTsV;

impl<'a> ParseCommand<'a> for qTsV {
    #[inline(always)]
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        if !buf.into_body().is_empty() {
            return None;
        }
        Some(qTsV)
    }
}
