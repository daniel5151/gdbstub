use super::prelude::*;

#[derive(PartialEq, Eq, Debug)]
pub struct qRcmd<'a> {
    pub hex_cmd: &'a [u8],
}

impl<'a> ParseCommand<'a> for qRcmd<'a> {
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        let body = buf.into_body_buf();
        match body {
            [] => Some(qRcmd { hex_cmd: &[] }),
            [b',', hex_cmd @ ..] => Some(qRcmd {
                hex_cmd: decode_hex(hex_cmd).ok()?,
            }),
            _ => None,
        }
    }
}
