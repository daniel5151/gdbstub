use super::prelude::*;

use crate::common::Pid;

#[derive(Debug)]
pub struct D {
    pub pid: Option<Pid>,
}

impl<'a> ParseCommand<'a> for D {
    #[inline(always)]
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        let body = buf.into_body();
        let pid = match body {
            [b';', pid @ ..] => Some(Pid::new(decode_hex(pid).ok()?)?),
            _ => None,
        };
        Some(D { pid })
    }
}
