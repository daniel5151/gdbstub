use super::prelude::*;

#[derive(Debug)]
pub struct qSupported<'a> {
    pub packet_buffer_len: usize,
    pub features: Features<'a>,
}

impl<'a> ParseCommand<'a> for qSupported<'a> {
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        let packet_buffer_len = buf.full_len();
        let body = buf.into_body();
        match body {
            [b':', body @ ..] => Some(qSupported {
                packet_buffer_len,
                features: Features(body),
            }),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct Features<'a>(&'a [u8]);

impl<'a> Features<'a> {
    pub fn into_iter(self) -> impl Iterator<Item = Result<Option<(Feature, bool)>, ()>> + 'a {
        self.0.split(|b| *b == b';').map(|s| match s.last() {
            None => Err(()),
            Some(&c) => match c {
                b'+' | b'-' => {
                    let feature = match &s[..s.len() - 1] {
                        b"multiprocess" => Feature::Multiprocess,
                        // TODO: implementing other features will require IDET plumbing
                        _ => return Ok(None),
                    };
                    Ok(Some((feature, c == b'+')))
                }
                _ => {
                    // TODO: add support for "xmlRegisters="
                    // that's the only feature packet that uses an '=', and AFAIK, it's not really
                    // used anymore...
                    Ok(None)
                }
            },
        })
    }
}

#[derive(Debug)]
pub enum Feature {
    Multiprocess,
}
