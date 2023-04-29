use super::prelude::*;

use crate::common::Signal;
use crate::protocol::common::hex::HexString;
use crate::protocol::common::thread_id::{SpecificThreadId, ThreadId};

// TODO?: instead of lazily parsing data, parse the strings into a compressed
// binary representations that can be stuffed back into the packet buffer and
// return an iterator over the binary data that's _guaranteed_ to be valid. This
// would clean up some of the code in the vCont handler.
//
// The interesting part would be to see whether or not the simplified error
// handing code will compensate for all the new code required to pre-validate
// the data...
#[derive(Debug)]
pub enum vCont<'a> {
    Query,
    Actions(Actions<'a>),
}

impl<'a> ParseCommand<'a> for vCont<'a> {
    #[inline(always)]
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
    FixedStep(SpecificThreadId),
    FixedCont(SpecificThreadId),
}

impl<'a> Actions<'a> {
    fn new_from_buf(buf: &'a [u8]) -> Actions<'a> {
        Actions::Buf(ActionsBuf(buf))
    }

    #[inline(always)]
    pub fn new_step(tid: SpecificThreadId) -> Actions<'a> {
        Actions::FixedStep(tid)
    }

    #[inline(always)]
    pub fn new_continue(tid: SpecificThreadId) -> Actions<'a> {
        Actions::FixedCont(tid)
    }

    pub fn iter(&self) -> impl Iterator<Item = Option<VContAction<'a>>> + '_ {
        match self {
            Actions::Buf(x) => EitherIter::A(x.iter()),
            Actions::FixedStep(x) => EitherIter::B(core::iter::once(Some(VContAction {
                kind: VContKind::Step,
                thread: Some(*x),
            }))),
            Actions::FixedCont(x) => EitherIter::B(core::iter::once(Some(VContAction {
                kind: VContKind::Continue,
                thread: Some(*x),
            }))),
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
    ContinueWithSig(Signal),
    RangeStep(HexString<'a>, HexString<'a>),
    Step,
    StepWithSig(Signal),
    Stop,
}

impl<'a> VContKind<'a> {
    #[inline(always)]
    fn from_bytes(s: &[u8]) -> Option<VContKind<'_>> {
        use self::VContKind::*;

        let res = match s {
            [b'c'] => Continue,
            [b's'] => Step,
            [b't'] => Stop,
            [b'C', sig @ ..] => ContinueWithSig(Signal(decode_hex(sig).ok()?)),
            [b'S', sig @ ..] => StepWithSig(Signal(decode_hex(sig).ok()?)),
            [b'r', range @ ..] => {
                let mut range = range.split(|b| *b == b',');
                RangeStep(HexString(range.next()?), HexString(range.next()?))
            }
            _ => return None,
        };

        Some(res)
    }
}

/// Helper type to unify iterators that output the same type. Returned as an
/// opaque type from `Actions::iter()`.
enum EitherIter<A, B> {
    A(A),
    B(B),
}

impl<A, B, T> Iterator for EitherIter<A, B>
where
    A: Iterator<Item = T>,
    B: Iterator<Item = T>,
{
    type Item = T;

    #[inline(always)]
    fn next(&mut self) -> Option<T> {
        match self {
            EitherIter::A(a) => a.next(),
            EitherIter::B(b) => b.next(),
        }
    }
}
