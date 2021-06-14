use super::prelude::*;
use crate::protocol::commands::ext::Breakpoints;

use crate::arch::{Arch, BreakpointKind};

enum CmdKind {
    Add,
    Remove,
}

impl<T: Target, C: Connection> GdbStubImpl<T, C> {
    #[inline(always)]
    fn handle_breakpoint_common(
        &mut self,
        ops: crate::target::ext::breakpoints::BreakpointsOps<T>,
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
            0 if ops.sw_breakpoint().is_some() => {
                let ops = ops.sw_breakpoint().unwrap();
                let bp_kind = bp_kind!();
                match cmd_kind {
                    CmdKind::Add => ops.add_sw_breakpoint(addr, bp_kind),
                    CmdKind::Remove => ops.remove_sw_breakpoint(addr, bp_kind),
                }
            }
            1 if ops.hw_breakpoint().is_some() => {
                let ops = ops.hw_breakpoint().unwrap();
                let bp_kind = bp_kind!();
                match cmd_kind {
                    CmdKind::Add => ops.add_hw_breakpoint(addr, bp_kind),
                    CmdKind::Remove => ops.remove_hw_breakpoint(addr, bp_kind),
                }
            }
            2 | 3 | 4 if ops.hw_watchpoint().is_some() => {
                use crate::target::ext::breakpoints::WatchKind;
                let kind = match cmd.type_ {
                    2 => WatchKind::Write,
                    3 => WatchKind::Read,
                    4 => WatchKind::ReadWrite,
                    _ => unreachable!(),
                };
                let len = <T::Arch as Arch>::Usize::from_be_bytes(cmd.kind)
                    .ok_or(Error::TargetMismatch)?;
                let ops = ops.hw_watchpoint().unwrap();
                match cmd_kind {
                    CmdKind::Add => ops.add_hw_watchpoint(addr, len, kind),
                    CmdKind::Remove => ops.remove_hw_watchpoint(addr, len, kind),
                }
            }
            // only 5 types defined by the protocol
            _ => return Ok(HandlerStatus::Handled),
        };

        match supported {
            Err(e) => {
                Err(e).handle_error()?;
                Ok(HandlerStatus::Handled)
            }
            Ok(true) => Ok(HandlerStatus::NeedsOk),
            Ok(false) => Err(Error::NonFatalError(22)),
        }
    }

    pub(crate) fn handle_breakpoints<'a>(
        &mut self,
        _res: &mut ResponseWriter<C>,
        target: &mut T,
        command: Breakpoints<'a>,
    ) -> Result<HandlerStatus, Error<T::Error, C::Error>> {
        let ops = match target.breakpoints() {
            Some(ops) => ops,
            None => return Ok(HandlerStatus::Handled),
        };

        crate::__dead_code_marker!("breakpoints", "impl");

        let handler_status = match command {
            Breakpoints::z(cmd) => self.handle_breakpoint_common(ops, cmd, CmdKind::Remove)?,
            Breakpoints::Z(cmd) => self.handle_breakpoint_common(ops, cmd, CmdKind::Add)?,
            // TODO: handle ZWithBytecode once agent expressions are implemented
            _ => HandlerStatus::Handled,
        };
        Ok(handler_status)
    }
}
