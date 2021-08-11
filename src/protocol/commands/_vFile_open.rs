use super::prelude::*;

use crate::target::ext::host_io::{HostIoOpenFlags, HostIoMode};

#[derive(Debug)]
pub struct vFileOpen<'a> {
    pub filename: &'a [u8],
    pub flags: HostIoOpenFlags,
    pub mode: HostIoMode,
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
                let flags = HostIoOpenFlags::from_bits(decode_hex(body.next()?).ok()?).unwrap();
                let mode = HostIoMode::from_bits(decode_hex(body.next()?).ok()?).unwrap();
                Some(vFileOpen{filename, flags, mode})
            },
            _ => None,
        }
    }
}
