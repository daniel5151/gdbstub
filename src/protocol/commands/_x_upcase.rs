use super::prelude::*;
use crate::protocol::common::hex::decode_bin_buf;

#[derive(Debug)]
pub struct X<'a> {
    pub addr: &'a [u8],
    pub val: &'a [u8],
}

impl<'a> ParseCommand<'a> for X<'a> {
    #[inline(always)]
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        let body = buf.into_body();

        let mut body = body.splitn_mut(3, |&b| b == b',' || b == b':');
        let addr = decode_hex_buf(body.next()?).ok()?;
        // See the comment in `_m_upcase.rs` for why the `len` field is handled
        // this way. All the same rationale applies here (given that the X
        // packet is just a new-and-improved version of the M packet).
        let _len: usize = decode_hex(body.next()?).ok()?;
        let val = decode_bin_buf(body.next()?)?;

        Some(X { addr, val })
    }
}
