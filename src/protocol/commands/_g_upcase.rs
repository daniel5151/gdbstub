use core::convert::TryFrom;

use crate::protocol::common::HexDecoder;

#[derive(PartialEq, Eq, Debug)]
pub struct G<'a> {
    pub vals: HexDecoder<'a>,
}

impl<'a> TryFrom<&'a str> for G<'a> {
    type Error = ();

    fn try_from(body: &'a str) -> Result<Self, ()> {
        Ok(G {
            vals: HexDecoder::new(body).map_err(drop)?,
        })
    }
}
