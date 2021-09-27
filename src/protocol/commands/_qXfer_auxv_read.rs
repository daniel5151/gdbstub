use super::prelude::*;

pub type qXferAuxvRead<'a> = QXferReadBase<'a, AuxvAnnex>;

#[derive(Debug)]
pub struct AuxvAnnex;

impl ParseAnnex for AuxvAnnex {
    fn from_buf(buf: &[u8]) -> Option<Self> {
        if buf != b"" {
            return None;
        }

        Some(AuxvAnnex)
    }
}
