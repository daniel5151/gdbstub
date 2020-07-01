use super::prelude::*;

#[derive(PartialEq, Eq, Debug)]
pub struct z {
    pub type_: u8,
    pub addr: u64,
    pub kind: u8,
}

impl<'a> ParseCommand<'a> for z {
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        let body = buf.into_body_str();
        let mut body = body.split(',');
        let type_ = u8::from_str_radix(body.next()?, 16).ok()?;
        let addr = u64::from_str_radix(body.next()?, 16).ok()?;
        let kind = u8::from_str_radix(body.next()?, 16).ok()?;

        Some(z { type_, addr, kind })
    }
}
