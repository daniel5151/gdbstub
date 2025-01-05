// use super::prelude::*; // unused
use crate::protocol::common::qxfer::ParseAnnex;
use crate::protocol::common::qxfer::QXferReadBase;

pub type qXferAuxvRead<'a> = QXferReadBase<'a, AuxvAnnex>;

#[derive(Debug)]
pub struct AuxvAnnex;

impl ParseAnnex<'_> for AuxvAnnex {
    #[inline(always)]
    fn from_buf(buf: &[u8]) -> Option<Self> {
        if buf != b"" {
            return None;
        }

        Some(AuxvAnnex)
    }
}
