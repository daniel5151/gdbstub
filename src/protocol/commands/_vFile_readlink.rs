use super::prelude::*;

#[derive(Debug)]
pub struct vFileReadlink<'a> {
    pub filename: &'a [u8],

    pub buf: &'a mut [u8],
}

impl<'a> ParseCommand<'a> for vFileReadlink<'a> {
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        let (buf, body_range) = buf.into_raw_buf();
        let (body, buf) = buf[body_range.start..].split_at_mut(body_range.end - body_range.start);

        if body.is_empty() {
            return None;
        }

        match body {
            [b':', body @ ..] => {
                let filename = decode_hex_buf(body).ok()?;
                Some(vFileReadlink{filename, buf})
            },
            _ => None,
        }
    }
}
