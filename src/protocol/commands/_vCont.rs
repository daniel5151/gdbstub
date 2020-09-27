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
pub struct Actions<'a>(&'a [u8]);

impl<'a> Actions<'a> {
    pub fn into_iter(self) -> impl Iterator<Item = Option<VContAction>> + 'a {
        self.0.split(|b| *b == b';').skip(1).map(|act| {
            let mut s = act.split(|b| *b == b':');
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

#[derive(PartialEq, Eq, Debug)]
pub struct VContAction {
    pub kind: VContKind,
    pub thread: Option<ThreadId>,
}

#[derive(PartialEq, Eq, Debug)]
pub enum VContKind {
    Continue,
    ContinueWithSig(u8),
    RangeStep(u64, u64),
    Step,
    StepWithSig(u8),
    Stop,
}

impl VContKind {
    fn from_bytes(s: &[u8]) -> Option<VContKind> {
        use self::VContKind::*;

        let mut s = s.split(|b| *b == b' ');
        let res = match s.next().unwrap() {
            b"c" => Continue,
            b"C" => ContinueWithSig(decode_hex(s.next()?).ok()?),
            b"s" => Step,
            b"S" => StepWithSig(decode_hex(s.next()?).ok()?),
            b"t" => Stop,
            b"r" => {
                let mut range = s.next()?.split(|b| *b == b',');
                let start = decode_hex(range.next()?).ok()?;
                let end = decode_hex(range.next()?).ok()?;
                RangeStep(start, end)
            }
            _ => return None,
        };

        Some(res)
    }
}
