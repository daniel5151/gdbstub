use gdbstub::target;
use gdbstub::target::ext::catch_syscalls::SyscallNumbers;

use crate::gdb::Emu;

// This implementation is for illustrative purposes only. If the target doesn't
// support syscalls then there is no need to implement this extension

impl target::ext::catch_syscalls::CatchSyscalls for Emu {
    fn enable_catch_syscalls(
        &mut self,
        _filter: Option<SyscallNumbers<u32>>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    fn disable_catch_syscalls(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}
