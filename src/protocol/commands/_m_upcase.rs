use super::prelude::*;

#[derive(Debug)]
pub struct M<'a> {
    pub addr: &'a [u8],
    pub val: &'a [u8],
}

impl<'a> ParseCommand<'a> for M<'a> {
    #[inline(always)]
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        let body = buf.into_body();

        let mut body = body.split_mut(|&b| b == b',' || b == b':');
        let addr = decode_hex_buf(body.next()?).ok()?;
        // The length part of the packet doesn't appear to actually be useful,
        // given that it can be trivially derived from the amount of data read
        // in via `val`.
        //
        // As such - we'll still parse it to ensure the packet is
        // spec-compliant, but we won't stash the parsed length anywhere.
        //
        // TODO?: dig into whether any GDB clients actually attempt to send over
        // a mismatched `len` and `val` (e.g: sending a longer `val`, with `len`
        // used to truncate the data). My gut feeling is that this would never
        // actually occur in practice (and the fact that `gdbstub` has gotten
        // away with not handling this for many years at this point reinforces
        // that gut feeling).
        let _len: usize = decode_hex(body.next()?).ok()?;
        let val = decode_hex_buf(body.next()?).ok()?;

        Some(M { addr, val })
    }
}
