use super::prelude::*;

#[derive(PartialEq, Eq, Debug)]
pub struct qAttached {
    pub pid: Option<isize>,
}

impl<'a> ParseCommand<'a> for qAttached {
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        let body = buf.into_body_str();
        Some(qAttached {
            pid: if body.is_empty() {
                None
            } else {
                Some(decode_hex(body.trim_start_matches(':').as_bytes()).ok()?)
            },
        })
    }
}
