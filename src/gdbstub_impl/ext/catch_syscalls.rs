use super::prelude::*;
use crate::{
    arch::Arch,
    protocol::commands::{_QCatchSyscalls::QCatchSyscalls, ext::CatchSyscalls},
    target::ext::catch_syscalls::SyscallNumbers,
};

impl<T: Target, C: Connection> GdbStubImpl<T, C> {
    pub(crate) fn handle_catch_syscalls(
        &mut self,
        _res: &mut ResponseWriter<C>,
        target: &mut T,
        command: CatchSyscalls,
    ) -> Result<HandlerStatus, Error<T::Error, C::Error>> {
        let ops = match target.catch_syscalls() {
            Some(ops) => ops,
            None => return Ok(HandlerStatus::Handled),
        };

        crate::__dead_code_marker!("catch_syscalls", "impl");

        let handler_status = match command {
            CatchSyscalls::QCatchSyscalls(cmd) => {
                match cmd {
                    QCatchSyscalls::Disable => {
                        ops.disable_catch_syscalls().map_err(Error::TargetError)?
                    }
                    QCatchSyscalls::Enable(sysno) => {
                        // FIXME: report integer overflow instead of silently ignoring
                        let mut filter = sysno
                            .into_iter()
                            .filter_map(|x| <T::Arch as Arch>::Usize::from_be_bytes(x));
                        ops.enable_catch_syscalls(Some(SyscallNumbers { inner: &mut filter }))
                            .map_err(Error::TargetError)?
                    }
                    QCatchSyscalls::EnableAll => ops
                        .enable_catch_syscalls(None)
                        .map_err(Error::TargetError)?,
                }
                HandlerStatus::NeedsOk
            }
        };

        Ok(handler_status)
    }
}
