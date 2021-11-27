use super::prelude::*;
use crate::protocol::commands::ext::CatchSyscalls;

use crate::arch::Arch;
use crate::protocol::commands::_QCatchSyscalls::QCatchSyscalls;
use crate::target::ext::catch_syscalls::SyscallNumbers;

impl<T: Target, C: Connection> GdbStubImpl<T, C> {
    pub(crate) fn handle_catch_syscalls(
        &mut self,
        _res: &mut ResponseWriter<C>,
        target: &mut T,
        command: CatchSyscalls,
    ) -> Result<HandlerStatus, Error<T::Error, C::Error>> {
        let ops = match target.support_catch_syscalls() {
            Some(ops) => ops,
            None => return Ok(HandlerStatus::Handled),
        };

        crate::__dead_code_marker!("catch_syscalls", "impl");

        let handler_status = match command {
            CatchSyscalls::QCatchSyscalls(cmd) => {
                match cmd {
                    QCatchSyscalls::Disable => ops.disable_catch_syscalls().handle_error()?,
                    QCatchSyscalls::Enable(sysno) => {
                        let mut error = false;
                        let mut filter = sysno
                            .into_iter()
                            .map(|x| <T::Arch as Arch>::Usize::from_be_bytes(x))
                            .take_while(|x| {
                                error = x.is_none();
                                !error
                            })
                            .flatten();
                        ops.enable_catch_syscalls(Some(SyscallNumbers { inner: &mut filter }))
                            .handle_error()?;
                        if error {
                            return Err(Error::TargetMismatch);
                        }
                    }
                    QCatchSyscalls::EnableAll => ops.enable_catch_syscalls(None).handle_error()?,
                }
                HandlerStatus::NeedsOk
            }
        };

        Ok(handler_status)
    }
}
