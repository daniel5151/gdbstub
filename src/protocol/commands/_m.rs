use core::convert::TryFrom;

#[derive(PartialEq, Eq, Debug)]
pub struct m {
    pub addr: u64,
    pub len: usize,
}

impl TryFrom<&str> for m {
    type Error = ();

    fn try_from(body: &str) -> Result<Self, ()> {
        let mut body = body.split(',');
        let addr = u64::from_str_radix(body.next().ok_or(())?, 16).map_err(drop)?;
        let len = usize::from_str_radix(body.next().ok_or(())?, 16).map_err(drop)?;

        Ok(m { addr, len })
    }
}
