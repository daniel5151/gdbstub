use super::prelude::*;

#[derive(Debug)]
pub struct vFileReadlink<'a> {
    pub filename: &'a [u8],
}

impl<'a> ParseCommand<'a> for vFileReadlink<'a> {
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        let body = buf.into_body();
        if body.is_empty() {
            return None;
        }

        match body {
            [b':', body @ ..] => {
                let filename = decode_hex_buf(body).ok()?;
                Some(vFileReadlink{filename})
            },
            _ => None,
        }
    }
}
