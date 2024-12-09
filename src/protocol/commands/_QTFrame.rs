use super::prelude::*;
use crate::target::ext::tracepoints::{Tracepoint, FrameRequest};

#[derive(Debug)]
pub struct QTFrame<'a>(pub FrameRequest<&'a mut [u8]>);

impl<'a> ParseCommand<'a> for QTFrame<'a> {
    #[inline(always)]
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        let body = buf.into_body();
        match body {
            [b':', body @ ..] => {
                let mut s = body.split_mut(|b| *b == b':');
                let selector = s.next()?;
                Some(match selector.as_ref() {
                    b"pc" => {
                        let addr = decode_hex_buf(s.next()?).ok()?;
                        QTFrame(FrameRequest::AtPC(addr))
                    },
                    b"tdp" => {
                        let tp = Tracepoint(decode_hex(s.next()?).ok()?);
                        QTFrame(FrameRequest::Hit(tp))
                    },
                    b"range" => {
                        let start = decode_hex_buf(s.next()?).ok()?;
                        let end = decode_hex_buf(s.next()?).ok()?;
                        QTFrame(FrameRequest::Between(start, end))
                    },
                    b"outside" => {
                        let start = decode_hex_buf(s.next()?).ok()?;
                        let end = decode_hex_buf(s.next()?).ok()?;
                        QTFrame(FrameRequest::Outside(start, end))
                    },
                    n => {
                        QTFrame(FrameRequest::Select(decode_hex(n).ok()?))
                    },
                })
            },
            _ => None,
        }
    }
}
