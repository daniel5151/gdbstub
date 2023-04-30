use crate::gdb::Emu;
use gdbstub::target;
use gdbstub::target::ext::catch_syscalls::SyscallNumbers;

// This implementation is for illustrative purposes only. If the target doesn't
// support syscalls then there is no need to implement this extension

impl target::ext::catch_syscalls::CatchSyscalls for Emu {
    fn enable_catch_syscalls(
        &mut self,
        filter: Option<SyscallNumbers<'_, u32>>,
    ) -> target::TargetResult<(), Self> {
        match filter {
            Some(numbers) => eprintln!(
                "Enabled catching syscalls: {:?}",
                numbers.collect::<Vec<u32>>()
            ),
            None => eprintln!("Enabled catching all syscalls"),
        }
        Ok(())
    }

    fn disable_catch_syscalls(&mut self) -> target::TargetResult<(), Self> {
        eprintln!("Disabled catching syscalls");
        Ok(())
    }
}
