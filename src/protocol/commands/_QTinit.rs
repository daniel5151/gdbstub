use super::prelude::*;

#[derive(Debug)]
pub struct QTinit;

impl<'a> ParseCommand<'a> for QTinit {
    #[inline(always)]
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        if !buf.into_body().is_empty() {
            return None;
        }
        Some(QTinit)
    }
}
