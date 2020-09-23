use super::prelude::*;

#[derive(Debug)]
pub struct QStartupWithShell {
    pub value: bool,
}

impl<'a> ParseCommand<'a> for QStartupWithShell {
    fn __protocol_hint(target: &mut impl Target) -> bool {
        if let Some(ops) = target.extended_mode() {
            return ops.configure_startup_shell().is_some();
        }
        false
    }

    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        let body = buf.into_body();
        let value = match body as &[u8] {
            b":0" => false,
            b":1" => true,
            _ => return None,
        };
        Some(QStartupWithShell { value })
    }
}
