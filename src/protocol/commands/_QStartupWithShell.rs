use super::prelude::*;

#[derive(Debug)]
pub struct QStartupWithShell {
    pub value: bool,
}

impl<'a> ParseCommand<'a> for QStartupWithShell {
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        let body = buf.into_body();
        let value = match body as &[u8] {
            b":0" => false,
            b":1" => true,
            _ => return None,
        };
        Some(QStartupWithShell { value })
    }
}
