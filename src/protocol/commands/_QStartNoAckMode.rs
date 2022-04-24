use super::prelude::*;

#[derive(Debug)]
pub struct QStartNoAckMode;

impl<'a> ParseCommand<'a> for QStartNoAckMode {
    #[inline(always)]
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        if !buf.into_body().is_empty() {
            return None;
        }
        Some(QStartNoAckMode)
    }
}
