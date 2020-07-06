use super::prelude::*;

#[derive(PartialEq, Eq, Debug)]
pub struct D {
    pub pid: Option<TidSelector>,
}

impl<'a> ParseCommand<'a> for D {
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        let body = buf.into_body_str();
        let pid = body.trim_start_matches(';').parse::<TidSelector>().ok();
        Some(D { pid })
    }
}
