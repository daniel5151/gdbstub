use super::prelude::*;

#[derive(PartialEq, Eq, Debug)]
pub struct c {
    pub addr: Option<u64>,
}

impl<'a> ParseCommand<'a> for c {
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        let body = buf.into_body_str();

        if body.is_empty() {
            return Some(c { addr: None });
        }
        let addr = u64::from_str_radix(body, 16).ok()?;
        Some(c { addr: Some(addr) })
    }
}
