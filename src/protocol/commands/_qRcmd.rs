use super::prelude::*;

#[derive(PartialEq, Eq, Debug)]
pub struct qRcmd<'a> {
    pub hex_cmd: &'a [u8],
}

impl<'a> ParseCommand<'a> for qRcmd<'a> {
    fn __protocol_hint(target: &mut impl Target) -> bool {
        target.monitor_cmd().is_some()
    }

    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        crate::__dead_code_marker!("qRcmd", "from_packet");

        let body = buf.into_body();
        match body {
            [] => Some(qRcmd { hex_cmd: &[] }),
            [b',', hex_cmd @ ..] => Some(qRcmd {
                hex_cmd: decode_hex_buf(hex_cmd).ok()?,
            }),
            _ => None,
        }
    }
}
