use super::prelude::*;

use crate::protocol::common::hex::decode_bin_buf;

#[derive(Debug)]
pub struct vFilePwrite<'a> {
    pub fd: u32,
    pub offset: &'a [u8],
    pub data: &'a [u8],
}

impl<'a> ParseCommand<'a> for vFilePwrite<'a> {
    #[inline(always)]
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        let body = buf.into_body();
        if body.is_empty() {
            return None;
        }

        match body {
            [b':', body @ ..] => {
                let mut body = body.splitn_mut(3, |b| *b == b',');
                let fd = decode_hex(body.next()?).ok()?;
                let offset = decode_hex_buf(body.next()?).ok()?;
                let data = decode_bin_buf(body.next()?)?;
                Some(vFilePwrite { fd, offset, data })
            }
            _ => None,
        }
    }
}
