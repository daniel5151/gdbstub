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
            _ => Some(vCont::Actions(Actions::new_from_buf(body))),
        }
    }
}

#[derive(Debug)]
pub enum Actions<'a> {
    Buf(ActionsBuf<'a>),
    Fixed(ActionsFixed),
}

impl<'a> Actions<'a> {
    fn new_from_buf(buf: &'a [u8]) -> Actions<'a> {
        Actions::Buf(ActionsBuf(buf))
    }

    pub fn new_step(tid: ThreadId) -> Actions<'a> {
        Actions::Fixed(ActionsFixed(VContAction {
            kind: VContKind::from_bytes(b"s").unwrap(),
            thread: Some(tid),
        }))
    }

    pub fn new_continue(tid: ThreadId) -> Actions<'a> {
        Actions::Fixed(ActionsFixed(VContAction {
            kind: VContKind::from_bytes(b"c").unwrap(),
            thread: Some(tid),
        }))
    }

    pub fn iter(&self) -> impl Iterator<Item = Option<VContAction>> + 'a {
        match self {
            Actions::Fixed(x) => EitherIter::Left(x.iter()),
            Actions::Buf(x) => EitherIter::Right(x.iter()),
        }
    }
}

#[derive(Debug)]
pub struct ActionsBuf<'a>(&'a [u8]);

impl<'a> ActionsBuf<'a> {
    fn iter(&self) -> impl Iterator<Item = Option<VContAction>> + 'a {
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

#[derive(Debug)]
pub struct ActionsFixed(VContAction);

impl ActionsFixed {
    fn iter(&self) -> impl Iterator<Item = Option<VContAction>> {
        Some(Some(self.0)).into_iter()
    }
}

#[derive(Debug, Copy, Clone)]
pub struct VContAction {
    pub kind: VContKind,
    pub thread: Option<ThreadId>,
}

#[derive(Debug, Copy, Clone)]
pub enum VContKind {
    Continue,
    ContinueWithSig(u8),
    // RangeStep(&'a [u8], &'a [u8]),
    Step,
    StepWithSig(u8),
    Stop,
}

impl VContKind {
    fn from_bytes(s: &[u8]) -> Option<VContKind> {
        use self::VContKind::*;

        let res = match s {
            [b'c'] => Continue,
            [b's'] => Step,
            [b't'] => Stop,
            [b'C', sig @ ..] => ContinueWithSig(decode_hex(sig).ok()?),
            [b'S', sig @ ..] => StepWithSig(decode_hex(sig).ok()?),
            // [b'r', range @ ..] => {
            //     let mut range = range.split_mut(|b| *b == b',');
            //     let start = decode_hex_buf(range.next()?).ok()?;
            //     let end = decode_hex_buf(range.next()?).ok()?;
            //     RangeStep(start, end)
            // }
            _ => return None,
        };

        Some(res)
    }
}

/// Helper type to unify iterators that output the same type. Returned as an
/// opaque type from `Actions::iter()`.
enum EitherIter<A, B> {
    Left(A),
    Right(B),
}

impl<A, B, T> Iterator for EitherIter<A, B>
where
    A: Iterator<Item = T>,
    B: Iterator<Item = T>,
{
    type Item = T;
    fn next(&mut self) -> Option<T> {
        match self {
            EitherIter::Left(a) => a.next(),
            EitherIter::Right(b) => b.next(),
        }
    }
}
