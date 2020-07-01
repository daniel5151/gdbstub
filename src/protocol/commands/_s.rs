use super::prelude::*;

#[derive(PartialEq, Eq, Debug)]
pub struct s {
    pub addr: Option<u64>,
}

impl<'a> ParseCommand<'a> for s {
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        let body = buf.into_body_str();
        if body.is_empty() {
            return Some(s { addr: None });
        }

        let addr = u64::from_str_radix(body, 16).ok()?;
        Some(s { addr: Some(addr) })
    }
}
