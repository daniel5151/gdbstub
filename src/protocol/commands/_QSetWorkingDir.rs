use super::prelude::*;

#[derive(Debug)]
pub struct QSetWorkingDir<'a> {
    pub dir: Option<&'a [u8]>,
}

impl<'a> ParseCommand<'a> for QSetWorkingDir<'a> {
    fn __protocol_hint(target: &mut impl Target) -> bool {
        if let Some(ops) = target.extended_mode() {
            return ops.configure_working_dir().is_some();
        }
        false
    }

    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        let body = buf.into_body();
        let dir = match body {
            [b':', dir @ ..] => match decode_hex_buf(dir).ok()? {
                [] => None,
                s => Some(s as &[u8]),
            },
            _ => return None,
        };

        Some(QSetWorkingDir { dir })
    }
}
