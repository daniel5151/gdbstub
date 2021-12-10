use crate::protocol::commands::ParseCommand;
use crate::protocol::common::hex::decode_hex;
use crate::protocol::packet::PacketBuf;

/// Parse the `annex` field of a qXfer packet. Used in conjunction with
/// `QXferBase` to cut keep qXfer packet parsing DRY.
pub trait ParseAnnex<'a>: Sized {
    fn from_buf(buf: &'a [u8]) -> Option<Self>;
}

#[derive(Debug)]
pub struct QXferReadBase<'a, T: ParseAnnex<'a>> {
    pub annex: T,
    pub offset: u64,
    pub length: usize,

    pub buf: &'a mut [u8],
}

impl<'a, T: ParseAnnex<'a>> ParseCommand<'a> for QXferReadBase<'a, T> {
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        let (buf, body_range) = buf.into_raw_buf();

        // this looks a bit wacky, but the compiler is dumb and won't elide bounds
        // checks without it.
        let (body, buf) = {
            let buf = buf.get_mut(body_range.start..)?;
            if body_range.end - body_range.start > buf.len() {
                return None;
            }
            buf.split_at_mut(body_range.end - body_range.start)
        };

        if body.is_empty() {
            return None;
        }

        let mut body = body.split(|b| *b == b':').skip(1);
        let annex = T::from_buf(body.next()?)?;

        let mut body = body.next()?.split(|b| *b == b',');
        let offset = decode_hex(body.next()?).ok()?;
        let length = decode_hex(body.next()?).ok()?;

        Some(QXferReadBase {
            annex,
            offset,
            length,
            buf,
        })
    }
}
