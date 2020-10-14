use super::prelude::*;

#[derive(Debug)]
pub struct Z<'a> {
    pub type_: u8,
    pub addr: &'a [u8],
    /// architecture dependent
    pub kind: u8,
    // TODO: Add support for breakpoint 'conds', 'persist', and 'cmds' feature
}

impl<'a> ParseCommand<'a> for Z<'a> {
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        let body = buf.into_body();
        let mut body = body.split_mut(|&b| b == b',');
        let type_ = decode_hex(body.next()?).ok()?;
        let addr = decode_hex_buf(body.next()?).ok()?;
        let kind = decode_hex(body.next()?).ok()?;

        // TODO: properly parse 'conds', 'persist', and 'cmds' fields in 'Z' packets

        Some(Z { type_, addr, kind })
    }
}
