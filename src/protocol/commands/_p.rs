use super::prelude::*;

#[derive(Debug)]
pub struct p<'a> {
    pub reg_id: usize,

    pub buf: &'a mut [u8],
}

impl<'a> ParseCommand<'a> for p<'a> {
    #[inline(always)]
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        let (buf, body_range) = buf.into_raw_buf();
        let body = buf.get(body_range.start..body_range.end)?;

        if body.is_empty() {
            return None;
        }

        let reg_id = decode_hex(body).ok()?;

        Some(p { reg_id, buf })
    }
}
