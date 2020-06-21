use core::convert::TryFrom;

#[derive(PartialEq, Eq, Debug)]
pub struct Z {
    pub type_: u8,
    // FIXME: 'Z' packets should use Target::USize for addr
    pub addr: u64,
    /// architecture dependent
    pub kind: u8,
    // TODO: Add support for breakpoint 'conds', 'persist', and 'cmds' feature
}

impl TryFrom<&str> for Z {
    type Error = ();

    fn try_from(body: &str) -> Result<Self, ()> {
        let mut body = body.split(',');
        let type_ = u8::from_str_radix(body.next().ok_or(())?, 16).map_err(drop)?;
        let addr = u64::from_str_radix(body.next().ok_or(())?, 16).map_err(drop)?;
        let kind = u8::from_str_radix(body.next().ok_or(())?, 16).map_err(drop)?;
        // TODO: properly parse 'conds', 'persist', and 'cmds' fields in 'Z' packets

        Ok(Z { type_, addr, kind })
    }
}
