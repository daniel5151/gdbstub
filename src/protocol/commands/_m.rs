use super::prelude::*;

#[derive(PartialEq, Eq, Debug)]
pub struct m {
    pub addr: u64,
    pub len: usize,
}

impl<'a> ParseCommand<'a> for m {
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        let body = buf.into_body_str();
        let mut body = body.split(',');
        let addr = u64::from_str_radix(body.next()?, 16).ok()?;
        let len = usize::from_str_radix(body.next()?, 16).ok()?;

        Some(m { addr, len })
    }
}
