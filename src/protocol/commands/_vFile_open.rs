use super::prelude::*;

use crate::target::ext::host_io::{HostIoOpenFlags, HostIoOpenMode};

#[derive(Debug)]
pub struct vFileOpen<'a> {
    pub filename: &'a [u8],
    pub flags: HostIoOpenFlags,
    pub mode: HostIoOpenMode,
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
                let flags = HostIoOpenFlags::from_bits_truncate(decode_hex(body.next()?).ok()?);
                let mode = HostIoOpenMode::from_bits_truncate(decode_hex(body.next()?).ok()?);
                Some(vFileOpen { filename, flags, mode })
            },
            _ => None,
        }
    }
}
