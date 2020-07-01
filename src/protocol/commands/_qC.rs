use super::prelude::*;

#[derive(PartialEq, Eq, Debug)]
pub struct qC;

impl<'a> ParseCommand<'a> for qC {
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        let body = buf.into_body_str();
        if !body.is_empty() {
            return None;
        }
        Some(qC)
    }
}
