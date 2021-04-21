use super::prelude::*;

#[derive(Debug)]
pub struct bc;

impl<'a> ParseCommand<'a> for bc {
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        if !buf.into_body().is_empty() {
            return None;
        }
        Some(bc)
    }
}
