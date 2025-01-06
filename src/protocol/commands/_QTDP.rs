use super::prelude::*;
use crate::target::ext::tracepoints::Tracepoint;

#[derive(Debug)]
pub enum QTDP<'a> {
    Create(CreateTDP<'a>),
    Define(DefineTDP<'a>),
}

#[derive(Debug)]
pub struct CreateTDP<'a> {
    pub number: Tracepoint,
    pub addr: &'a [u8],
    pub enable: bool,
    pub step: u64,
    pub pass: u64,
    pub fast: Option<&'a [u8]>,
    pub condition: Option<&'a [u8]>,
    pub more: bool,
}

#[derive(Debug)]
pub struct DefineTDP<'a> {
    pub number: Tracepoint,
    pub addr: &'a [u8],
    pub while_stepping: bool,
    pub actions: &'a mut [u8],
}

impl<'a> ParseCommand<'a> for QTDP<'a> {
    #[inline(always)]
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        let body = buf.into_body();
        match body {
            [b':', b'-', actions @ ..] => {
                let mut params = actions.splitn_mut(4, |b| *b == b':');
                let number = Tracepoint(decode_hex(params.next()?).ok()?);
                let addr = decode_hex_buf(params.next()?).ok()?;
                let actions = params.next()?;
                Some(QTDP::Define(DefineTDP {
                    number,
                    addr,
                    while_stepping: false,
                    actions
                }))
            },
            [b':', tracepoint @ ..] => {
                // Strip off the trailing '-' that indicates if there will be
                // more packets after this
                let (tracepoint, more) = match tracepoint {
                    [rest @ .., b'-'] => (rest, true),
                    x => (x, false)
                };
                let mut params = tracepoint.splitn_mut(6, |b| *b == b':');
                let n = Tracepoint(decode_hex(params.next()?).ok()?);
                let addr = decode_hex_buf(params.next()?).ok()?;
                let ena = params.next()?;
                let step = decode_hex(params.next()?).ok()?;
                let pass_and_end = params.next()?;
                let pass = decode_hex(pass_and_end).ok()?;
                // TODO: parse `F` fast tracepoint options
                // TODO: parse `X` tracepoint conditions
                let _options = params.next();
                return Some(QTDP::Create(CreateTDP {
                    number: n,
                    addr,
                    enable: match ena { [b'E'] => Some(true), [b'D'] => Some(false), _ => None }?,
                    step,
                    pass,
                    more,
                    // TODO: populate fast tracepoint options
                    // TODO: populate tracepoint conditions
                    fast: None,
                    condition: None,
                }))
            },
            _ => None,
        }
    }
}
