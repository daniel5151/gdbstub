use crate::protocol::common::qxfer::ParseAnnex;
use crate::protocol::common::qxfer::QXferReadBase;

pub type qXferLibrariesRead<'a> = QXferReadBase<'a, LibrariesAnnex>;

#[derive(Debug)]
pub struct LibrariesAnnex;

impl<'a> ParseAnnex<'a> for LibrariesAnnex {
    #[inline(always)]
    fn from_buf(buf: &[u8]) -> Option<Self> {
        if buf != b"" {
            return None;
        }

        Some(LibrariesAnnex)
    }
}
