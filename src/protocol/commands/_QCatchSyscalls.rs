use super::prelude::*;

#[derive(Debug)]
pub enum QCatchSyscalls<'a> {
    Disable,
    Enable(lists::ArgListHex<'a>),
    EnableAll,
}

impl<'a> ParseCommand<'a> for QCatchSyscalls<'a> {
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        let body = buf.into_body();

        match body {
            [b':', b'0'] => Some(QCatchSyscalls::Disable),
            [b':', b'1', b';', sysno @ ..] => Some(QCatchSyscalls::Enable(
                lists::ArgListHex::from_packet(sysno)?,
            )),
            [b':', b'1'] => Some(QCatchSyscalls::EnableAll),
            _ => None,
        }
    }
}
