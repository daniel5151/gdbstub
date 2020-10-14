use super::prelude::*;

// TODO: instead of parsing lazily when invoked, parse the strings into a
// compressed binary representations that can be stuffed back into the packet
// buffer, and return an iterator over the binary data that's _guaranteed_ to be
// valid. This would clean up some of the code in the vCont handler.
#[derive(Debug)]
pub enum vCont<'a> {
    Query,
    Actions(Actions<'a>),
}

impl<'a> ParseCommand<'a> for vCont<'a> {
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        let body = buf.into_body();
        match body as &[u8] {
            b"?" => Some(vCont::Query),
            _ => Some(vCont::Actions(Actions(body))),
        }
    }
}

/// A lazily evaluated iterator over the actions specified in a vCont packet.
#[derive(Debug)]
pub struct Actions<'a>(&'a mut [u8]);

impl<'a> Actions<'a> {
    pub fn into_iter(self) -> impl Iterator<Item = Option<VContAction<'a>>> + 'a {
        self.0.split_mut(|b| *b == b';').skip(1).map(|act| {
            let mut s = act.split_mut(|b| *b == b':');
            let kind = s.next()?;
            let thread = match s.next() {
                Some(s) => Some(s.try_into().ok()?),
                None => None,
            };

            Some(VContAction {
                kind: VContKind::from_bytes(kind)?,
                thread,
            })
        })
    }
}

#[derive(Debug)]
pub struct VContAction<'a> {
    pub kind: VContKind<'a>,
    pub thread: Option<ThreadId>,
}

#[derive(Debug)]
pub enum VContKind<'a> {
    Continue,
    ContinueWithSig(u8),
    RangeStep(&'a [u8], &'a [u8]),
    Step,
    StepWithSig(u8),
    Stop,
}

impl<'a> VContKind<'a> {
    fn from_bytes(s: &mut [u8]) -> Option<VContKind> {
        use self::VContKind::*;

        let res = match s {
            [b'c'] => Continue,
            [b's'] => Step,
            [b't'] => Stop,
            [b'C', sig @ ..] => ContinueWithSig(decode_hex(sig).ok()?),
            [b'S', sig @ ..] => StepWithSig(decode_hex(sig).ok()?),
            [b'r', range @ ..] => {
                let mut range = range.split_mut(|b| *b == b',');
                let start = decode_hex_buf(range.next()?).ok()?;
                let end = decode_hex_buf(range.next()?).ok()?;
                RangeStep(start, end)
            }
            _ => return None,
        };

        Some(res)
    }
}
