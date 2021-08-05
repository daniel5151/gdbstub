use super::prelude::*;

#[derive(Debug)]
pub struct vFileFstat {
    pub fd: i32,
}

impl<'a> ParseCommand<'a> for vFileFstat {
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        let body = buf.into_body();
        if body.is_empty() {
            return None;
        }

        match body {
            [b':', body @ ..] => {
                let mut body = body.splitn_mut_no_panic(3, |b| *b == b',');
                let fd = decode_hex(body.next()?).ok()?;
                Some(vFileFstat{fd})
            },
            _ => None,
        }
    }
}
