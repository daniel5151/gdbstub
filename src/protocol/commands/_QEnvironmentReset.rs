use super::prelude::*;

#[derive(Debug)]
pub struct QEnvironmentReset;

impl<'a> ParseCommand<'a> for QEnvironmentReset {
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        if !buf.into_body().is_empty() {
            return None;
        }
        Some(QEnvironmentReset)
    }
}
