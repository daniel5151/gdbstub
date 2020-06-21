use core::convert::TryFrom;

#[derive(PartialEq, Eq, Debug)]
pub struct z {
    pub type_: u8,
    // FIXME: 'z' packets should use Target::USize for addr
    pub addr: u64,
    pub kind: u8,
}

impl TryFrom<&str> for z {
    type Error = ();

    fn try_from(body: &str) -> Result<Self, ()> {
        let mut body = body.split(',');
        let type_ = u8::from_str_radix(body.next().ok_or(())?, 16).map_err(drop)?;
        let addr = u64::from_str_radix(body.next().ok_or(())?, 16).map_err(drop)?;
        let kind = u8::from_str_radix(body.next().ok_or(())?, 16).map_err(drop)?;

        Ok(z { type_, addr, kind })
    }
}
