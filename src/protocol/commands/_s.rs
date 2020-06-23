use core::convert::TryFrom;

#[derive(PartialEq, Eq, Debug)]
pub struct s {
    pub addr: Option<u64>,
}

impl TryFrom<&str> for s {
    type Error = ();

    fn try_from(body: &str) -> Result<Self, ()> {
        if body.is_empty() {
            return Ok(s { addr: None });
        }

        let addr = u64::from_str_radix(body, 16).map_err(drop)?;
        Ok(s { addr: Some(addr) })
    }
}
