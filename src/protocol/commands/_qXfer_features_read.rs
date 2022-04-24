// use super::prelude::*; // unused

use crate::protocol::common::qxfer::{ParseAnnex, QXferReadBase};

pub type qXferFeaturesRead<'a> = QXferReadBase<'a, FeaturesAnnex<'a>>;

#[derive(Debug)]
pub struct FeaturesAnnex<'a> {
    pub name: &'a [u8],
}

impl<'a> ParseAnnex<'a> for FeaturesAnnex<'a> {
    #[inline(always)]
    fn from_buf(buf: &'a [u8]) -> Option<Self> {
        Some(FeaturesAnnex { name: buf })
    }
}
