use super::prelude::*;

#[derive(Debug)]
pub struct QAgent {
    pub value: bool,
}

impl<'a> ParseCommand<'a> for QAgent {
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        let body = buf.into_body();
        let value = match body as &[u8] {
            b":0" => false,
            b":1" => true,
            _ => return None,
        };
        Some(QAgent { value })
    }
}
