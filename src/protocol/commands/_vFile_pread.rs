use super::prelude::*;

#[derive(Debug)]
pub struct vFilePread<'a> {
    pub fd: u32,
    pub count: usize,
    pub offset: u64,

    pub buf: &'a mut [u8],
}

impl<'a> ParseCommand<'a> for vFilePread<'a> {
    #[inline(always)]
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        let (buf, body_range) = buf.into_raw_buf();
        let body = buf.get_mut(body_range.start..body_range.end)?;

        if body.is_empty() {
            return None;
        }

        match body {
            [b':', body @ ..] => {
                let mut body = body.splitn_mut_no_panic(3, |b| *b == b',');
                let fd = decode_hex(body.next()?).ok()?;
                let count = decode_hex(body.next()?).ok()?;
                let offset = decode_hex(body.next()?).ok()?;

                drop(body);

                Some(vFilePread {
                    fd,
                    count,
                    offset,
                    buf,
                })
            }
            _ => None,
        }
    }
}
