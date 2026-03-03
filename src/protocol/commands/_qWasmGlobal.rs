use super::prelude::*;

#[derive(Debug)]
pub struct qWasmGlobal {
    pub frame: u32,
    pub global: u32,
}

impl<'a> ParseCommand<'a> for qWasmGlobal {
    #[inline(always)]
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        let body = buf.into_body();
        if body.is_empty() || body[0] != b':' {
            return None;
        }
        let mut parts = body[1..].split(|b| *b == b';');
        let frame = parts.next()?;
        let frame = str::from_utf8(frame).ok()?.parse::<u32>().ok()?;
        let global = parts.next()?;
        let global = str::from_utf8(global).ok()?.parse::<u32>().ok()?;
        if parts.next().is_some() {
            // Too many parameters.
            return None;
        }
        Some(qWasmGlobal { frame, global })
    }
}
