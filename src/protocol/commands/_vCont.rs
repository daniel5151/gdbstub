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
            _ => {
                for range in body
                    .split_mut(|b| *b == b'r')
                    .skip(1)
                    .flat_map(|s| s.split_mut(|b| *b == b':').take(1))
                {
                    let mut range = range.split_mut(|b| *b == b',');
                    let _ = decode_hex_buf(range.next()?).ok()?;
                    let _ = decode_hex_buf(range.next()?).ok()?;
                }
                Some(vCont::Actions(Actions::new_from_buf(body)))
            }
        }
    }
}

#[derive(Debug)]
pub enum Actions<'a> {
    Buf(ActionsBuf<'a>),
    FixedStep(SpecificThreadId),
    FixedCont(SpecificThreadId),
}

impl<'a> Actions<'a> {
    fn new_from_buf(buf: &'a [u8]) -> Actions<'a> {
        Actions::Buf(ActionsBuf(buf))
    }

    pub fn new_step(tid: SpecificThreadId) -> Actions<'a> {
        Actions::FixedStep(tid)
    }

    pub fn new_continue(tid: SpecificThreadId) -> Actions<'a> {
        Actions::FixedCont(tid)
    }

    pub fn iter(&self) -> impl Iterator<Item = Option<VContAction<'a>>> + '_ {
        match self {
            Actions::Buf(x) => EitherIter::A(x.iter()),
            Actions::FixedStep(x) => EitherIter::B(
                Some(Some(VContAction {
                    kind: VContKind::Step,
                    thread: Some(*x),
                }))
                .into_iter(),
            ),
            Actions::FixedCont(x) => EitherIter::C(
                Some(Some(VContAction {
                    kind: VContKind::Continue,
                    thread: Some(*x),
                }))
                .into_iter(),
            ),
        }
    }
}

#[derive(Debug)]
pub struct ActionsBuf<'a>(&'a [u8]);

impl<'a> ActionsBuf<'a> {
    fn iter(&self) -> impl Iterator<Item = Option<VContAction<'a>>> + '_ {
        self.0.split(|b| *b == b';').skip(1).map(|act| {
            let mut s = act.split(|b| *b == b':');
            let kind = s.next()?;
            let thread = match s.next() {
                Some(s) => Some(SpecificThreadId::try_from(ThreadId::try_from(s).ok()?).ok()?),
                None => None,
            };

            Some(VContAction {
                kind: VContKind::from_bytes(kind)?,
                thread,
            })
        })
    }
}

#[derive(Debug, Copy, Clone)]
pub struct VContAction<'a> {
    pub kind: VContKind<'a>,
    pub thread: Option<SpecificThreadId>,
}

#[derive(Debug, Copy, Clone)]
pub enum VContKind<'a> {
    Continue,
    ContinueWithSig(u8),
    RangeStep(&'a [u8], &'a [u8]),
    Step,
    StepWithSig(u8),
    Stop,
}

impl<'a> VContKind<'a> {
    fn from_bytes(s: &[u8]) -> Option<VContKind> {
        use self::VContKind::*;

        let res = match s {
            [b'c'] => Continue,
            [b's'] => Step,
            [b't'] => Stop,
            [b'C', sig @ ..] => ContinueWithSig(decode_hex(sig).ok()?),
            [b'S', sig @ ..] => StepWithSig(decode_hex(sig).ok()?),
            [b'r', range @ ..] => {
                // relies on the fact that start and end were decoded as part of
                // the initial packet parse.
                let mut range = range.split(|b| *b == b',');
                let start = {
                    let s = range.next()?;
                    &s[..(s.len() / 2)]
                };
                let end = {
                    let s = range.next()?;
                    &s[..(s.len() / 2)]
                };
                RangeStep(start, end)
            }
            _ => return None,
        };

        Some(res)
    }
}

/// Helper type to unify iterators that output the same type. Returned as an
/// opaque type from `Actions::iter()`.
enum EitherIter<A, B, C> {
    A(A),
    B(B),
    C(C),
}

impl<A, B, C, T> Iterator for EitherIter<A, B, C>
where
    A: Iterator<Item = T>,
    B: Iterator<Item = T>,
    C: Iterator<Item = T>,
{
    type Item = T;
    fn next(&mut self) -> Option<T> {
        match self {
            EitherIter::A(a) => a.next(),
            EitherIter::B(b) => b.next(),
            EitherIter::C(b) => b.next(),
        }
    }
}
