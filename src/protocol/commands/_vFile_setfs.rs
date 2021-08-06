use super::prelude::*;
use crate::target::ext::host_io::FsKind;
use core::num::NonZeroUsize;

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
                let fs = match decode_hex(body).ok()? {
                    0 => FsKind::Stub,
                    pid => FsKind::Pid(NonZeroUsize::new(pid).unwrap()),
                };
                Some(vFileSetfs{fs})
            },
            _ => None,
        }
    }
}
