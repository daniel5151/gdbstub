use super::prelude::*;

#[derive(Debug)]
pub struct vFileFstat {
    pub fd: u32,
}

impl<'a> ParseCommand<'a> for vFileFstat {
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        let body = buf.into_body();
        if body.is_empty() {
            return None;
        }

        match body {
            [b':', body @ ..] => {
                let fd = decode_hex(body).ok()?;
                Some(vFileFstat{fd})
            },
            _ => None,
        }
    }
}
