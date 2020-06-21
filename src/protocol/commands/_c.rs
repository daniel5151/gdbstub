use core::convert::TryFrom;

#[derive(PartialEq, Eq, Debug)]
pub struct c {
    // FIXME: 'c' packet's addr should correspond to Target::USize
    pub addr: Option<u64>,
}

impl TryFrom<&str> for c {
    type Error = ();

    fn try_from(body: &str) -> Result<Self, ()> {
        if body.is_empty() {
            return Ok(c { addr: None });
        }
        let addr = u64::from_str_radix(body, 16).map_err(drop)?;
        Ok(c { addr: Some(addr) })
    }
}
