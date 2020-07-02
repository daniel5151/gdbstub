use super::prelude::*;

#[derive(PartialEq, Eq, Debug)]
pub struct qfThreadInfo;

impl<'a> ParseCommand<'a> for qfThreadInfo {
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        if !buf.into_body().is_empty() {
            return None;
        }
        Some(qfThreadInfo)
    }
}
