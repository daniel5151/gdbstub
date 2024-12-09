use super::prelude::*;
use crate::target::ext::tracepoints::{BufferShape, TraceBuffer};

#[derive(Debug)]
pub enum QTBuffer<'a>
{
    Request { offset: u64, length: usize, data: &'a mut [u8] },
    Configure { buffer: TraceBuffer },
}

impl<'a> ParseCommand<'a> for QTBuffer<'a> {
    #[inline(always)]
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        let (buf, body_range) = buf.into_raw_buf();
        let body = &buf[body_range];
        match body {
            [b':', body @ ..] => {
                let mut s = body.split(|b| *b == b':');
                let opt = s.next()?;
                Some(match opt.as_ref() {
                    b"circular" => {
                        let shape = s.next()?;
                        QTBuffer::Configure { buffer: TraceBuffer::Shape(match shape {
                            [b'1'] => Some(BufferShape::Circular),
                            [b'0'] => Some(BufferShape::Linear),
                            _ => None,
                        }?)}
                    },
                    b"size" => {
                        let size = s.next()?;
                        QTBuffer::Configure { buffer: TraceBuffer::Size(match size {
                            [b'-', b'1'] => None,
                            i => Some(decode_hex(i).ok()?)
                        })}
                    },
                    req => {
                        let mut req_opts = req.split(|b| *b == b',');
                        let (offset, length) = (req_opts.next()?, req_opts.next()?);
                        let offset = decode_hex(offset).ok()?;
                        let length = decode_hex(length).ok()?;
                        // Our response has to be a hex encoded buffer that fits within
                        // our packet size, which means we actually have half as much space
                        // as our slice would indicate.
                        let (front, _back) = buf.split_at_mut(buf.len() / 2);
                        QTBuffer::Request { offset, length, data: front } 
                    },
                })

            },
            _ => None,
        }
    }
}
