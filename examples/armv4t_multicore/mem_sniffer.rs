use armv4t_emu::Memory;

#[derive(Debug)]
pub enum AccessKind {
    Read,
    Write,
}

#[derive(Debug)]
pub struct Access {
    pub kind: AccessKind,
    pub addr: u32,
    pub val: u32,
    pub len: usize,
}

/// Wraps a `Memory` object, logging any accesses with the provided callback.
#[derive(Debug)]
pub struct MemSniffer<'a, M, F: FnMut(Access)> {
    mem: &'a mut M,
    addrs: &'a [u32],
    on_access: F,
}

impl<'a, M: Memory, F: FnMut(Access)> MemSniffer<'a, M, F> {
    pub fn new(mem: &'a mut M, addrs: &'a [u32], on_access: F) -> MemSniffer<'a, M, F> {
        MemSniffer {
            mem,
            addrs,
            on_access,
        }
    }
}

macro_rules! impl_memsniff_r {
    ($fn:ident, $ret:ty) => {
        fn $fn(&mut self, addr: u32) -> $ret {
            let ret = self.mem.$fn(addr);
            if self.addrs.contains(&addr) {
                (self.on_access)(Access {
                    kind: AccessKind::Read,
                    addr,
                    val: ret as u32,
                    len: ret.to_le_bytes().len(),
                });
            }
            ret
        }
    };
}

macro_rules! impl_memsniff_w {
    ($fn:ident, $val:ty) => {
        fn $fn(&mut self, addr: u32, val: $val) {
            self.mem.$fn(addr, val);
            if self.addrs.contains(&addr) {
                (self.on_access)(Access {
                    kind: AccessKind::Write,
                    addr,
                    val: val as u32,
                    len: val.to_le_bytes().len(),
                });
            }
        }
    };
}

impl<'a, M: Memory, F: FnMut(Access)> Memory for MemSniffer<'a, M, F> {
    impl_memsniff_r!(r8, u8);
    impl_memsniff_r!(r16, u16);
    impl_memsniff_r!(r32, u32);
    impl_memsniff_w!(w8, u8);
    impl_memsniff_w!(w16, u16);
    impl_memsniff_w!(w32, u32);
}
