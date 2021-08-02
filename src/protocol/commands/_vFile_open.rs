use super::prelude::*;

#[derive(Debug)]
pub struct vFileOpen<'a> {
    pub filename: &'a [u8],
    pub flags: u64,
    pub mode: u64,
}

impl<'a> ParseCommand<'a> for vFileOpen<'a> {
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        let body = buf.into_body();
        if body.is_empty() {
            return None;
        }

        match body {
            [b':', body @ ..] => {
                let mut body = body.splitn_mut_no_panic(3, |b| *b == b',');
                let filename = decode_hex_buf(body.next()?).ok()?;
                let flags= decode_hex(body.next()?).ok()?;
                let mode= decode_hex(body.next()?).ok()?;
                Some(vFileOpen{filename, flags, mode})
            },
            _ => None,
        }
    }
}
