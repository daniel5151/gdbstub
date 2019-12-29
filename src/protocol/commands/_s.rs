#[derive(PartialEq, Eq, Debug)]
pub struct s {
    // FIXME: 's' packet's addr should correspond to Target::USize
    pub addr: Option<u64>,
}

impl s {
    pub fn parse(body: &str) -> Result<Self, ()> {
        if body.is_empty() {
            return Ok(s { addr: None });
        }

        let addr = u64::from_str_radix(body, 16).map_err(drop)?;
        Ok(s { addr: Some(addr) })
    }
}
