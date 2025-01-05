// use super::prelude::*; // unused
use crate::protocol::common::qxfer::ParseAnnex;
use crate::protocol::common::qxfer::QXferReadBase;

pub type qXferMemoryMapRead<'a> = QXferReadBase<'a, MemoryMapAnnex>;

#[derive(Debug)]
pub struct MemoryMapAnnex;

impl ParseAnnex<'_> for MemoryMapAnnex {
    #[inline(always)]
    fn from_buf(buf: &[u8]) -> Option<Self> {
        if buf != b"" {
            return None;
        }

        Some(MemoryMapAnnex)
    }
}
