use super::prelude::*;

#[derive(Debug)]
pub struct vFileSetfs {
    pub fd: usize,
}

impl<'a> ParseCommand<'a> for vFileSetfs {
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        let body = buf.into_body();
        if body.is_empty() {
            return None;
        }

        match body {
            [b':', body @ ..] => {
                let fd = decode_hex(body).ok()?;
                Some(vFileSetfs{fd})
            },
            _ => None,
        }
    }
}
