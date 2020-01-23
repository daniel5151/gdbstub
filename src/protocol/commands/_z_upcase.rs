use alloc::vec::Vec;

#[derive(PartialEq, Eq, Debug)]
pub struct Z {
    pub type_: u8,
    // FIXME: 'Z' packets should use Target::USize for addr
    pub addr: u64,
    /// architecture dependent
    pub kind: u8,
    // TODO: Add support for breakpoint 'conds', 'persist', and 'cmds' feature
    pub conds: Vec<Vec<u8>>,
    pub persist: bool,
    pub cmds: Vec<Vec<u8>>,
}

impl Z {
    pub fn parse(body: &str) -> Result<Self, ()> {
        let mut body = body.split(',');
        let type_ = u8::from_str_radix(body.next().ok_or(())?, 16).map_err(drop)?;
        let addr = u64::from_str_radix(body.next().ok_or(())?, 16).map_err(drop)?;
        let kind = u8::from_str_radix(body.next().ok_or(())?, 16).map_err(drop)?;

        // TODO: properly parse 'conds', 'persist', and 'cmds' fields in 'Z' packets
        let conds = Vec::new();
        let persist = false;
        let cmds = Vec::new();

        Ok(Z {
            type_,
            addr,
            kind,
            conds,
            persist,
            cmds,
        })
    }
}
