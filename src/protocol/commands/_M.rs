use alloc::vec::Vec;

#[derive(PartialEq, Eq, Debug)]
pub struct M {
    // FIXME: 'M' packet's addr should correspond to Target::USize
    pub addr: u64,
    pub len: usize,
    pub val: Vec<u8>,
}

impl M {
    pub fn parse(body: &str) -> Result<Self, ()> {
        let mut body = body.split(|c| c == ',' || c == ':');
        let addr = u64::from_str_radix(body.next().ok_or(())?, 16).map_err(drop)?;
        let len = usize::from_str_radix(body.next().ok_or(())?, 16).map_err(drop)?;
        let val = body.next().ok_or(())?;

        if val.len() % 2 != 0 || !val.is_ascii() {
            return Err(());
        }

        let val = val
            .as_bytes()
            .chunks_exact(2)
            .map(|c| unsafe { core::str::from_utf8_unchecked(c) })
            .map(|c| u8::from_str_radix(c, 16))
            .collect::<Result<Vec<_>, _>>()
            .map_err(drop)?;

        Ok(M { addr, len, val })
    }
}
