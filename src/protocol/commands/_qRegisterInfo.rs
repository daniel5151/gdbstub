use super::prelude::*;

#[derive(Debug)]
pub struct qRegisterInfo {
    pub reg_id: usize,
}

impl<'a> ParseCommand<'a> for qRegisterInfo {
    #[inline(always)]
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        let body = buf.into_body();

        let reg_id = decode_hex(body).ok()?;

        Some(qRegisterInfo { reg_id })
    }
}
