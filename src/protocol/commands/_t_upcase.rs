use super::prelude::*;

#[derive(PartialEq, Eq, Debug)]
pub struct T {
    pub thread: ThreadId,
}

impl<'a> ParseCommand<'a> for T {
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        let body = buf.into_body_str();
        let thread = body.parse::<ThreadId>().ok()?;
        Some(T { thread })
    }
}
