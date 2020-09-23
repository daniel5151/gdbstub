use super::prelude::*;

#[derive(Debug)]
pub struct QEnvironmentUnset<'a> {
    pub key: &'a [u8],
}

impl<'a> ParseCommand<'a> for QEnvironmentUnset<'a> {
    fn __protocol_hint(target: &mut impl Target) -> bool {
        if let Some(ops) = target.extended_mode() {
            return ops.configure_env().is_some();
        }
        false
    }

    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        let body = buf.into_body();
        let key = match body {
            [b':', key @ ..] => decode_hex_buf(key).ok()?,
            _ => return None,
        };

        Some(QEnvironmentUnset { key })
    }
}
