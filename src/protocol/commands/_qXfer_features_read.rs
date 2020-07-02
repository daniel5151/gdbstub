use super::prelude::*;

#[derive(PartialEq, Eq, Debug)]
pub struct qXferFeaturesRead<'a> {
    pub annex: &'a str,
    pub offset: usize,
    pub len: usize,
}

impl<'a> ParseCommand<'a> for qXferFeaturesRead<'a> {
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        let body = buf.into_body_str();

        // body should be ":<target>:<offset>,<len>b"
        log::debug!("{}", body);
        if body.is_empty() {
            return None;
        }

        let mut body = body.split(':').skip(1);
        let annex = body.next()?;

        let mut body = body.next()?.split(',');
        let offset = decode_hex(body.next()?.as_bytes()).ok()?;
        let len = decode_hex(body.next()?.as_bytes()).ok()?;

        Some(qXferFeaturesRead { annex, offset, len })
    }
}
