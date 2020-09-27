use super::prelude::*;

#[derive(Debug)]
pub struct k;

impl<'a> ParseCommand<'a> for k {
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        if !buf.into_body().is_empty() {
            return None;
        }
        Some(k)
    }
}
