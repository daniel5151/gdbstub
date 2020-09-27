use super::prelude::*;

#[derive(Debug)]
pub struct qOffsets;

impl<'a> ParseCommand<'a> for qOffsets {
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        crate::__dead_code_marker!("qOffsets", "from_packet");

        if !buf.into_body().is_empty() {
            return None;
        }
        Some(qOffsets)
    }
}
