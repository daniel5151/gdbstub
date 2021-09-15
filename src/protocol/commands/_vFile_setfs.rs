use super::prelude::*;

use crate::target::ext::host_io::FsKind;

#[derive(Debug)]
pub struct vFileSetfs {
    pub fs: FsKind,
}

impl<'a> ParseCommand<'a> for vFileSetfs {
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        let body = buf.into_body();
        if body.is_empty() {
            return None;
        }

        match body {
            [b':', body @ ..] => {
                let fs = match crate::common::Pid::new(decode_hex(body).ok()?) {
                    None => FsKind::Stub,
                    Some(pid) => FsKind::Pid(pid),
                };
                Some(vFileSetfs{fs})
            },
            _ => None,
        }
    }
}
