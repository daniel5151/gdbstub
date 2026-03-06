use super::prelude::*;

#[derive(Debug)]
pub struct qWasmStackValue {
    pub frame: usize,
    pub index: usize,
}

impl<'a> ParseCommand<'a> for qWasmStackValue {
    #[inline(always)]
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        let body = buf.into_body();
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
        Some(qWasmStackValue { frame, index })
    }
}
