use super::prelude::*;

#[derive(PartialEq, Eq, Debug)]
pub struct R;

impl<'a> ParseCommand<'a> for R {
    fn __protocol_hint(target: &mut impl Target) -> bool {
        target.extended_mode().is_some()
    }

    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        crate::__dead_code_marker!("R", "from_packet");

        // Technically speaking, the `R` packet does include a hex-encoded byte as well,
        // but even the GDB docs mention that it's unused (heck, the source-level
        // comments in the GDB client suggest no-one really knows what it's used for).
        //
        // We'll pay some lip-service to this requirement by checking the body's length,
        // but we won't actually parse the number.
        let body = buf.into_body();
        if body.len() != 2 {
            None
        } else {
            Some(R)
        }
    }
}
