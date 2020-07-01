use super::prelude::*;

#[derive(PartialEq, Eq, Debug)]
pub struct Z {
    pub type_: u8,
    pub addr: u64,
    /// architecture dependent
    pub kind: u8,
    // TODO: Add support for breakpoint 'conds', 'persist', and 'cmds' feature
}

impl<'a> ParseCommand<'a> for Z {
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        let body = buf.into_body_str();
        let mut body = body.split(',');
        let type_ = u8::from_str_radix(body.next()?, 16).ok()?;
        let addr = u64::from_str_radix(body.next()?, 16).ok()?;
        let kind = u8::from_str_radix(body.next()?, 16).ok()?;
        // TODO: properly parse 'conds', 'persist', and 'cmds' fields in 'Z' packets

        Some(Z { type_, addr, kind })
    }
}
