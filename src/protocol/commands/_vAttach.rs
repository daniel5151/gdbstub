use super::prelude::*;

#[derive(PartialEq, Eq, Debug)]
pub struct vAttach {
    pub pid: Pid,
}

impl<'a> ParseCommand<'a> for vAttach {
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        crate::__dead_code_marker!("vAttach", "from_packet");

        let body = buf.into_body();
        let pid = match body {
            [b';', pid @ ..] => Pid::new(decode_hex(pid).ok()?)?,
            _ => return None,
        };
        Some(vAttach { pid })
    }
}
