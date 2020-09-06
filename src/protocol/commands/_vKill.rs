use super::prelude::*;

#[derive(PartialEq, Eq, Debug)]
pub struct vKill {
    pub pid: IdKind,
}

impl<'a> ParseCommand<'a> for vKill {
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        let body = buf.into_body_str();
        let pid = body.trim_start_matches(';').parse::<IdKind>().ok()?;
        Some(vKill { pid })
    }
}
