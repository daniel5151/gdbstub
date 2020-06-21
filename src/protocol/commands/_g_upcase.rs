use core::convert::TryFrom;

use crate::util::hexiter::HexIter;

#[derive(PartialEq, Eq, Debug)]
pub struct G<'a> {
    vals: HexIter<'a>,
}

impl<'a> TryFrom<&'a str> for G<'a> {
    type Error = ();

    fn try_from(body: &'a str) -> Result<Self, ()> {
        Ok(G {
            vals: HexIter::new(body).ok_or(())?,
        })
    }
}

impl<'a> Iterator for G<'a> {
    type Item = u8;

    fn next(&mut self) -> Option<u8> {
        self.vals.next()
    }
}
