use super::prelude::*;

#[derive(PartialEq, Eq, Debug)]
pub struct qsThreadInfo;

impl<'a> ParseCommand<'a> for qsThreadInfo {
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        if !buf.into_body().is_empty() {
            return None;
        }
        Some(qsThreadInfo)
    }
}
