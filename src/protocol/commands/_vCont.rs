use super::prelude::*;
use crate::common::Signal;
use crate::protocol::common::hex::HexString;
use crate::protocol::common::thread_id::SpecificThreadId;
use crate::protocol::common::thread_id::ThreadId;
use crate::protocol::IdKind;

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
                Some(s) => {
                    let mut tid = ThreadId::try_from(s).ok()?;

                    // Based on my (somewhat superficial) readings of the
                    // `gdbserver` and `gdb` codebases, it doesn't seem like
                    // there's any consensus on how vCont packets with a TID of
                    // `Any` should be be handled.
                    //
                    // `gdbserver` delegates the handling of this packet to
                    // individual targets, and in-turn, it seems most targets
                    // don't actually do any special handling of the 'Any' TID.
                    // They'll explicitly check for the 'All' TID, but then
                    // proceed to erroneously treat the 'Any' TID as though it
                    // was simply a request for a TID with ID of '0' to be
                    // resumed (which is prohibited by the GDB RSP spec).
                    //
                    // This behavior makes sense, given the context that AFAIK,
                    // upstream GDB never actually sends vCont packets with a
                    // 'Any' TID! Instead, upstream GDB will first query the
                    // remote to obtain a list of valid TIDs, and then send over
                    // a vCont packet with a specific TID selected.

                    // So, with all that said - how should `gdbstub` handle
                    // these sorts of packets?
                    //
                    // Unfortunately, it seems like there are some weird
                    // third-party GDB clients out there that do in-fact send an
                    // 'Any' TID as part of a vCont packet.
                    //
                    // See issue #150 for more info.
                    //
                    // As a workaround for these weird GDB clients, `gdbstub`
                    // takes the pragmatic approach of treating this request as
                    // though it the client requested _all_ threads to be
                    // resumed.
                    //
                    // If this turns out to be wrong... `gdbstub` can explore a
                    // more involved fix, whereby we bubble-up this `Any`
                    // request to the stub code itself, whereupon the stub can
                    // attempt to pick a "reasonable" TID to individually
                    // resume.
                    if tid.tid == IdKind::Any {
                        tid.tid = IdKind::All;
                    }

                    Some(SpecificThreadId::try_from(tid).ok()?)
                }
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

impl VContKind<'_> {
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
