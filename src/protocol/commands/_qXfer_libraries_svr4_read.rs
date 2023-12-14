use crate::protocol::common::qxfer::ParseAnnex;
use crate::protocol::common::qxfer::QXferReadBase;

pub type qXferLibrariesSvr4Read<'a> = QXferReadBase<'a, LibrariesSvr4Annex>;

#[derive(Debug)]
pub struct LibrariesSvr4Annex;

impl<'a> ParseAnnex<'a> for LibrariesSvr4Annex {
    #[inline(always)]
    fn from_buf(buf: &[u8]) -> Option<Self> {
        if buf != b"" {
            return None;
        }

        Some(LibrariesSvr4Annex)
    }
}
