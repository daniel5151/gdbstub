use super::prelude::*;

#[derive(Debug)]
pub struct QTStop;

impl<'a> ParseCommand<'a> for QTStop {
    #[inline(always)]
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        if !buf.into_body().is_empty() {
            return None;
        }
        Some(QTStop)
    }
}
