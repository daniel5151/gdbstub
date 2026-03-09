use super::prelude::*;

#[derive(Debug)]
pub struct qWasmStackValue<'a> {
    pub frame: usize,
    pub index: usize,
    pub buf: &'a mut [u8],
}

impl<'a> ParseCommand<'a> for qWasmStackValue<'a> {
    #[inline(always)]
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        let (buf, body_range) = buf.into_raw_buf();
        let body = buf.get(body_range.start..body_range.end)?;

        if body.is_empty() || body[0] != b':' {
            return None;
        }
        let mut parts = body[1..].split(|b| *b == b';');
        let frame = parts.next()?;
        let frame = str::from_utf8(frame).ok()?.parse::<usize>().ok()?;
        let index = parts.next()?;
        let index = str::from_utf8(index).ok()?.parse::<usize>().ok()?;
        if parts.next().is_some() {
            // Too many parameters.
            return None;
        }

        Some(qWasmStackValue {
            frame,
            index,
            buf,
        })
    }
}
