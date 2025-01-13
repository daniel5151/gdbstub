use super::prelude::*;
use crate::arch::Arch;
use crate::arch::BreakpointKind;
use crate::protocol::commands::ext::Breakpoints;
use maybe_async::maybe_async;

enum CmdKind {
    Add,
    Remove,
}

#[maybe_async(AFIT)]
impl<T: Target, C: Connection> GdbStubImpl<T, C> {
    #[inline(always)]
    async fn handle_breakpoint_common(
        &mut self,
        ops: crate::target::ext::breakpoints::BreakpointsOps<'_, T>,
        cmd: crate::protocol::commands::breakpoint::BasicBreakpoint<'_>,
        cmd_kind: CmdKind,
    ) -> Result<HandlerStatus, Error<T::Error, C::Error>> {
        let addr =
            <T::Arch as Arch>::Usize::from_be_bytes(cmd.addr).ok_or(Error::TargetMismatch)?;

        macro_rules! bp_kind {
            () => {
                BeBytes::from_be_bytes(cmd.kind)
                    .and_then(<T::Arch as Arch>::BreakpointKind::from_usize)
                    .ok_or(Error::TargetMismatch)?
            };
        }

        let supported = match cmd.type_ {
            0 if ops.support_sw_breakpoint().is_some() => {
                let ops = ops.support_sw_breakpoint().unwrap();
                let bp_kind = bp_kind!();
                match cmd_kind {
                    CmdKind::Add => ops.add_sw_breakpoint(addr, bp_kind),
                    CmdKind::Remove => ops.remove_sw_breakpoint(addr, bp_kind),
                }
                .await
            }
            1 if ops.support_hw_breakpoint().is_some() => {
                let ops = ops.support_hw_breakpoint().unwrap();
                let bp_kind = bp_kind!();
                match cmd_kind {
                    CmdKind::Add => ops.add_hw_breakpoint(addr, bp_kind).await,
                    CmdKind::Remove => ops.remove_hw_breakpoint(addr, bp_kind).await,
                }
            }
            2 | 3 | 4 if ops.support_hw_watchpoint().is_some() => {
                use crate::target::ext::breakpoints::WatchKind;
                let kind = match cmd.type_ {
                    2 => WatchKind::Write,
                    3 => WatchKind::Read,
                    4 => WatchKind::ReadWrite,
                    #[allow(clippy::unreachable)] // will be optimized out
                    _ => unreachable!(),
                };
                let len = <T::Arch as Arch>::Usize::from_be_bytes(cmd.kind)
                    .ok_or(Error::TargetMismatch)?;
                let ops = ops.support_hw_watchpoint().unwrap();
                match cmd_kind {
                    CmdKind::Add => ops.add_hw_watchpoint(addr, len, kind).await,
                    CmdKind::Remove => ops.remove_hw_watchpoint(addr, len, kind).await,
                }
            }
            // explicitly handle unguarded variants of known breakpoint types
            0 | 1 | 2 | 3 | 4 => return Ok(HandlerStatus::Handled),
            // warn if the GDB client ever sends a type outside the known types
            other => {
                warn!("unknown breakpoint type: {}", other);
                return Ok(HandlerStatus::Handled);
            }
        };

        match supported.handle_error()? {
            true => Ok(HandlerStatus::NeedsOk),
            false => Err(Error::NonFatalError(22)),
        }
    }

    pub(crate) async fn handle_breakpoints(
        &mut self,
        _res: &mut ResponseWriter<'_, C>,
        target: &mut T,
        command: Breakpoints<'_>,
    ) -> Result<HandlerStatus, Error<T::Error, C::Error>> {
        let ops = match target.support_breakpoints() {
            Some(ops) => ops,
            None => return Ok(HandlerStatus::Handled),
        };

        crate::__dead_code_marker!("breakpoints", "impl");

        let handler_status = match command {
            Breakpoints::z(cmd) => {
                self.handle_breakpoint_common(ops, cmd, CmdKind::Remove)
                    .await?
            }
            Breakpoints::Z(cmd) => {
                self.handle_breakpoint_common(ops, cmd, CmdKind::Add)
                    .await?
            }
            // TODO: handle ZWithBytecode once agent expressions are implemented
            _ => HandlerStatus::Handled,
        };
        Ok(handler_status)
    }
}
