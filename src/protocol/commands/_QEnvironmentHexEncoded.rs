use super::prelude::*;

#[derive(Debug)]
pub struct QEnvironmentHexEncoded<'a> {
    pub key: &'a [u8],
    pub value: Option<&'a [u8]>,
}

impl<'a> ParseCommand<'a> for QEnvironmentHexEncoded<'a> {
    #[inline(always)]
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        let body = buf.into_body();

        let (key, value) = match body {
            [b':', keyval @ ..] => {
                let keyval = decode_hex_buf(keyval).ok()?;
                let mut keyval = keyval.splitn(2, |b| *b == b'=');
                let key = keyval.next()?;
                let value = match keyval.next()? {
                    [] => None,
                    s => Some(s),
                };
                (key, value)
            }
            _ => return None,
        };

        Some(QEnvironmentHexEncoded { key, value })
    }
}
