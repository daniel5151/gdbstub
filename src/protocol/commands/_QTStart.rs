use super::prelude::*;

#[derive(Debug)]
pub struct QTStart;

impl<'a> ParseCommand<'a> for QTStart {
    #[inline(always)]
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        if !buf.into_body().is_empty() {
            return None;
        }
        Some(QTStart)
    }
}
