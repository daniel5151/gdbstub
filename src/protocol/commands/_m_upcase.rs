use core::convert::TryFrom;

use crate::util::HexDecoder;

#[derive(PartialEq, Eq, Debug)]
pub struct M<'a> {
    pub addr: u64,
    pub len: usize,
    pub val: HexDecoder<'a>,
}

impl<'a> TryFrom<&'a str> for M<'a> {
    type Error = ();

    fn try_from(body: &'a str) -> Result<Self, ()> {
        let mut body = body.split(|c| c == ',' || c == ':');
        let addr = u64::from_str_radix(body.next().ok_or(())?, 16).map_err(drop)?;
        let len = usize::from_str_radix(body.next().ok_or(())?, 16).map_err(drop)?;
        let val = body.next().ok_or(())?;

        Ok(M {
            addr,
            len,
            val: HexDecoder::new(val).map_err(drop)?,
        })
    }
}
