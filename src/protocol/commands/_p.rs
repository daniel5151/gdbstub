use super::prelude::*;

#[derive(PartialEq, Eq, Debug)]
pub struct p {
    pub reg_id: usize,
}

impl<'a> ParseCommand<'a> for p {
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        let reg_id = decode_hex(buf.into_body()).ok()?;
        Some(p { reg_id })
    }
}
