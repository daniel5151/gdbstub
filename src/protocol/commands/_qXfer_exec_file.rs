use super::prelude::*;
use crate::common::Pid;
use crate::protocol::common::qxfer::ParseAnnex;
use crate::protocol::common::qxfer::QXferReadBase;

pub type qXferExecFileRead<'a> = QXferReadBase<'a, ExecFileAnnex>;

#[derive(Debug)]
pub struct ExecFileAnnex {
    pub pid: Option<Pid>,
}

impl<'a> ParseAnnex<'a> for ExecFileAnnex {
    #[inline(always)]
    fn from_buf(buf: &[u8]) -> Option<Self> {
        let pid = match buf {
            [] => None,
            buf => Some(Pid::new(decode_hex(buf).ok()?)?),
        };

        Some(ExecFileAnnex { pid })
    }
}
