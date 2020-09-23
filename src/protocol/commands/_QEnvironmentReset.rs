use super::prelude::*;

#[derive(Debug)]
pub struct QEnvironmentReset;

impl<'a> ParseCommand<'a> for QEnvironmentReset {
    fn __protocol_hint(target: &mut impl Target) -> bool {
        if let Some(ops) = target.extended_mode() {
            return ops.configure_env().is_some();
        }
        false
    }

    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        if !buf.into_body().is_empty() {
            return None;
        }
        Some(QEnvironmentReset)
    }
}
