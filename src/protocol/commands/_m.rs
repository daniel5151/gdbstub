use super::prelude::*;

#[derive(Debug)]
pub struct m<'a> {
    pub addr: &'a [u8],
    pub len: usize,

    pub buf: &'a mut [u8],
}

impl<'a> ParseCommand<'a> for m<'a> {
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
        let body = &mut buf[body_range.start..];

        // should return 3 slices: the addr (hex-encoded), len (hex-encoded), and the
        // "rest" of the buffer
        let mut body = body.split_mut(|b| *b == b',' || *b == b'#');

        let addr = decode_hex_buf(body.next()?).ok()?;
        let addr_len = addr.len();
        let len = decode_hex(body.next()?).ok()?;

        drop(body);

        let (addr, buf) = buf.split_at_mut(body_range.start + addr_len);
        let addr = &addr[b"$m".len()..];

        Some(m { addr, len, buf })
    }
}
