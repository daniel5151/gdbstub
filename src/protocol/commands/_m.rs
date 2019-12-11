/// 'm addr,length'
///
/// Read length addressable memory units starting at address addr (see
/// addressable memory unit). Note that addr may not be aligned to any
/// particular boundary.
///
/// The stub need not use any particular size or alignment when gathering data
/// from memory for the response; even if addr is word-aligned and length is a
/// multiple of the word size, the stub is free to use byte accesses, or not.
/// For this reason, this packet may not be suitable for accessing memory-mapped
/// I/O devices.
///
/// Reply:
///
/// 'XX…'
/// Memory contents; each byte is transmitted as a two-digit hexadecimal number.
/// The reply may contain fewer addressable memory units than requested if the
/// server was able to read only part of the region of memory.
///
/// 'E NN'
/// NN is errno
#[derive(PartialEq, Eq, Debug)]
pub struct m {
    // FIXME: 'm' packet's addr should correspond to Target::USize
    pub addr: u64,
    pub len: usize,
}

impl m {
    pub fn parse(body: &str) -> Result<Self, ()> {
        let mut body = body.split(',');
        let addr = u64::from_str_radix(body.next().ok_or(())?, 16).map_err(drop)?;
        let len = usize::from_str_radix(body.next().ok_or(())?, 16).map_err(drop)?;

        Ok(m { addr, len })
    }
}
