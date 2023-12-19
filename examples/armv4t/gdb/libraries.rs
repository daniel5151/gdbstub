use super::copy_range_to_buf;
use crate::emu::Emu;
use gdbstub::target;
use gdbstub::target::TargetResult;

impl target::ext::libraries::Libraries for Emu {
    fn get_libraries(
        &self,
        offset: u64,
        length: usize,
        buf: &mut [u8],
    ) -> TargetResult<usize, Self> {
        if true {
            panic!()
        }
        let xml = r#"
<library-list>
  <library name="/test.elf">
    <segment address="0"/>
  </library>
</library-list>
"#
        .trim()
        .as_bytes();
        Ok(copy_range_to_buf(xml, offset, length, buf))
    }
}

impl target::ext::libraries::LibrariesSvr4 for Emu {
    fn get_libraries_svr4(
        &self,
        offset: u64,
        length: usize,
        buf: &mut [u8],
    ) -> TargetResult<usize, Self> {
        // `l_ld` is the address of the `PT_DYNAMIC` ELF segment, so fake an
        // address here.
        //
        // The `main-lm`, `lm`, and `lmid` seem to refer to in-memory structures
        // which gdb may read, but gdb also seems to work well enough if they're
        // null-ish or otherwise pointing to non-present things.
        let xml = r#"
<library-list-svr4 version="1.0" main-lm="0x4">
    <library name="/test.elf" lm="0x8" l_addr="0" l_ld="0" lmid="0x14"/>
</library-list-svr4>
"#
        .trim()
        .as_bytes();
        Ok(copy_range_to_buf(xml, offset, length, buf))
    }
}
