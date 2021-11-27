use super::prelude::*;

use crate::common::Pid;

#[derive(Debug)]
pub struct vKill {
    pub pid: Pid,
}

impl<'a> ParseCommand<'a> for vKill {
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        let body = buf.into_body();
        let pid = match body {
            [b';', pid @ ..] => Pid::new(decode_hex(pid).ok()?)?,
            _ => return None,
        };
        Some(vKill { pid })
    }
}
