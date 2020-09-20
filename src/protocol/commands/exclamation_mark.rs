use super::prelude::*;

#[derive(PartialEq, Eq, Debug)]
pub struct ExclamationMark;

impl<'a> ParseCommand<'a> for ExclamationMark {
    fn __protocol_hint(target: &mut impl Target) -> bool {
        target.extended_mode().is_some()
    }

    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        if !buf.into_body().is_empty() {
            return None;
        }
        Some(ExclamationMark)
    }
}
