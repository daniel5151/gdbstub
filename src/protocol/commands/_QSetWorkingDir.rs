use super::prelude::*;

#[derive(Debug)]
pub struct QSetWorkingDir<'a> {
    pub dir: Option<&'a [u8]>,
}

impl<'a> ParseCommand<'a> for QSetWorkingDir<'a> {
    #[inline(always)]
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
