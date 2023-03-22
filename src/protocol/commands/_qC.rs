use super::prelude::*;

#[derive(Debug)]
pub struct qC;

impl<'a> ParseCommand<'a> for qC {
    #[inline(always)]
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        if !buf.into_body().is_empty() {
            return None;
        }
        Some(qC)
    }
}
