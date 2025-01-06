use super::prelude::*;
use crate::target::ext::tracepoints::{BufferShape, TraceBuffer};

#[derive(Debug)]
pub struct QTBuffer(pub TraceBuffer);

impl ParseCommand<'_> for QTBuffer {
    #[inline(always)]
    fn from_packet(buf: PacketBuf<'_>) -> Option<Self> {
        match buf.into_body() {
            [b':', body @ ..] => {
                let mut s = body.splitn_mut(2, |b| *b == b':');
                let opt = s.next()?;
                match opt.as_ref() {
                    b"circular" => {
                        let shape = s.next()?;
                        Some(QTBuffer(TraceBuffer::Shape(match shape {
                            [b'1'] => Some(BufferShape::Circular),
                            [b'0'] => Some(BufferShape::Linear),
                            _ => None,
                        }?)))
                    },
                    b"size" => {
                        let size = s.next()?;
                        Some(QTBuffer(TraceBuffer::Size(match size {
                            [b'-', b'1'] => None,
                            i => Some(decode_hex(i).ok()?)
                        })))
                    },
                    _ => None,
                }

            },
            _ => None,
        }
    }
}
