use super::prelude::*;

#[derive(PartialEq, Eq, Debug)]
pub struct qOffsets;

impl<'a> ParseCommand<'a> for qOffsets {
    fn __protocol_hint(target: &mut impl Target) -> bool {
        target.section_offsets().is_some()
    }

    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        crate::__dead_code_marker!("qOffsets", "from_packet");

        if !buf.into_body().is_empty() {
            return None;
        }
        Some(qOffsets)
    }
}
