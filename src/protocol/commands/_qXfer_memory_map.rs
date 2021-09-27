use super::prelude::*;

pub type qXferMemoryMapRead<'a> = QXferReadBase<'a, MemoryMapAnnex>;

#[derive(Debug)]
pub struct MemoryMapAnnex;

impl ParseAnnex for MemoryMapAnnex {
    fn from_buf(buf: &[u8]) -> Option<Self> {
        if buf != b"" {
            return None;
        }

        Some(MemoryMapAnnex)
    }
}
