use super::prelude::*;

#[derive(Debug)]
pub struct vFlashDone;

impl<'a> ParseCommand<'a> for vFlashDone {
    #[inline(always)]
    fn from_packet(_buf: PacketBuf<'a>) -> Option<Self> {
        Some(vFlashDone)
    }
}
