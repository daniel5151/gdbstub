use super::prelude::*;

#[derive(Debug)]
pub struct QEnvironmentUnset<'a> {
    pub key: &'a [u8],
}

impl<'a> ParseCommand<'a> for QEnvironmentUnset<'a> {
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        let body = buf.into_body();
        let key = match body {
            [b':', key @ ..] => decode_hex_buf(key).ok()?,
            _ => return None,
        };

        Some(QEnvironmentUnset { key })
    }
}
