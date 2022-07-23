use super::prelude::*;

#[derive(Debug)]
pub struct m<'a> {
    pub addr: &'a [u8],
    pub len: usize,

    pub buf: &'a mut [u8],
}

impl<'a> ParseCommand<'a> for m<'a> {
    #[inline(always)]
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        // the total packet buffer currently looks like:
        //
        // +------+--------------------+-------------------+-------+-----------------+
        // | "$m" | addr (hex-encoded) | len (hex-encoded) | "#XX" | empty space ... |
        // +------+--------------------+-------------------+-------+-----------------+
        //
        // Unfortunately, while `len` can be hex-decoded right here and now into a
        // `usize`, `addr` corresponds to a Target::Arch::Usize, which requires holding
        // on to a valid &[u8] reference into the buffer.
        //
        // While it's not _perfectly_ efficient, simply leaving the decoded addr in
        // place and wasting a couple bytes is probably the easiest way to tackle this
        // problem:
        //
        // +------+------------------+------------------------------------------------+
        // | "$m" | addr (raw bytes) | usable buffer ...                              |
        // +------+------------------+------------------------------------------------+

        let (buf, body_range) = buf.into_raw_buf();
        let body = buf.get_mut(body_range.start..body_range.end)?;

        let mut body = body.split_mut_no_panic(|b| *b == b',');

        let addr = decode_hex_buf(body.next()?).ok()?;
        let addr_len = addr.len();
        let len = decode_hex(body.next()?).ok()?;

        // ensures that `split_at_mut` doesn't panic
        if buf.len() < body_range.start + addr_len {
            return None;
        }

        let (addr, buf) = buf.split_at_mut(body_range.start + addr_len);
        let addr = addr.get(b"$m".len()..)?;

        Some(m { addr, len, buf })
    }
}
