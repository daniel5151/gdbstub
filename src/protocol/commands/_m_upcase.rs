use super::prelude::*;

#[derive(PartialEq, Eq, Debug)]
pub struct M<'a> {
    pub addr: u64,
    pub len: usize,
    // TODO: replace HexDecoder with a decode_hex() &[u8]
    pub val: HexDecoder<'a>,
}

impl<'a> ParseCommand<'a> for M<'a> {
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        let body = buf.into_body_str();
        let mut body = body.split(|c| c == ',' || c == ':');
        let addr = u64::from_str_radix(body.next()?, 16).ok()?;
        let len = usize::from_str_radix(body.next()?, 16).ok()?;
        let val = body.next()?;

        Some(M {
            addr,
            len,
            val: HexDecoder::new(val).ok()?,
        })
    }
}
