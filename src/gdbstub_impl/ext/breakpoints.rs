use super::prelude::*;
use crate::protocol::commands::ext::Breakpoints;

use crate::arch::{Arch, BreakpointKind};
use crate::target::ext::breakpoints::BreakpointBytecodeKind;

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
        let kind =
            <T::Arch as Arch>::BreakpointKind::from_usize(cmd.kind).ok_or(Error::TargetMismatch)?;

        let handler_status = match cmd_kind {
            CmdKind::Add => {
                use crate::target::ext::breakpoints::WatchKind::*;
                let supported = match cmd.type_ {
                    0 => (ops.sw_breakpoint()).map(|op| op.add_sw_breakpoint(addr, kind)),
                    1 => (ops.hw_breakpoint()).map(|op| op.add_hw_breakpoint(addr, kind)),
                    2 => (ops.hw_watchpoint()).map(|op| op.add_hw_watchpoint(addr, Write)),
                    3 => (ops.hw_watchpoint()).map(|op| op.add_hw_watchpoint(addr, Read)),
                    4 => (ops.hw_watchpoint()).map(|op| op.add_hw_watchpoint(addr, ReadWrite)),
                    // only 5 types in the protocol
                    _ => None,
                };

                match supported {
                    None => HandlerStatus::Handled,
                    Some(Err(e)) => {
                        Err(e).handle_error()?;
                        HandlerStatus::Handled
                    }
                    Some(Ok(true)) => HandlerStatus::NeedsOk,
                    Some(Ok(false)) => return Err(Error::NonFatalError(22)),
                }
            }
            CmdKind::Remove => {
                use crate::target::ext::breakpoints::WatchKind::*;
                let supported = match cmd.type_ {
                    0 => (ops.sw_breakpoint()).map(|op| op.remove_sw_breakpoint(addr, kind)),
                    1 => (ops.hw_breakpoint()).map(|op| op.remove_hw_breakpoint(addr, kind)),
                    2 => (ops.hw_watchpoint()).map(|op| op.remove_hw_watchpoint(addr, Write)),
                    3 => (ops.hw_watchpoint()).map(|op| op.remove_hw_watchpoint(addr, Read)),
                    4 => (ops.hw_watchpoint()).map(|op| op.remove_hw_watchpoint(addr, ReadWrite)),
                    // only 5 types in the protocol
                    _ => None,
                };

                match supported {
                    None => HandlerStatus::Handled,
                    Some(Err(e)) => {
                        Err(e).handle_error()?;
                        HandlerStatus::Handled
                    }
                    Some(Ok(true)) => HandlerStatus::NeedsOk,
                    Some(Ok(false)) => return Err(Error::NonFatalError(22)),
                }
            }
        };

        Ok(handler_status)
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

        let handler_status = match command {
            Breakpoints::z(cmd) => {
                let addr = <T::Arch as Arch>::Usize::from_be_bytes(cmd.addr)
                    .ok_or(Error::TargetMismatch)?;

                let status = self.handle_breakpoint_common(ops, cmd, CmdKind::Remove)?;

                if let Some(agent_ops) = ops.breakpoint_agent() {
                    agent_ops
                        .clear_breakpoint_bytecode(BreakpointBytecodeKind::Command, addr)
                        .handle_error()?;
                    agent_ops
                        .clear_breakpoint_bytecode(BreakpointBytecodeKind::Condition, addr)
                        .handle_error()?;
                }

                status
            }
            Breakpoints::Z(cmd) => self.handle_breakpoint_common(ops, cmd, CmdKind::Add)?,

            Breakpoints::ZWithBytecode(cmd) if ops.breakpoint_agent().is_some() => {
                let addr = <T::Arch as Arch>::Usize::from_be_bytes(cmd.base.addr)
                    .ok_or(Error::TargetMismatch)?;

                let status = self.handle_breakpoint_common(ops, cmd.base, CmdKind::Add)?;
                let agent_ops = ops.breakpoint_agent().unwrap();

                if let Some(conds) = cmd.conds {
                    for bytecode in conds.into_iter() {
                        let bytecode = bytecode.ok_or(Error::PacketParse(
                            crate::protocol::PacketParseError::MalformedCommand,
                        ))?;

                        let id = agent_ops.register_bytecode(bytecode).handle_error()?;
                        agent_ops
                            .add_breakpoint_bytecode(
                                BreakpointBytecodeKind::Condition,
                                addr,
                                id,
                                false, // dummy value
                            )
                            .handle_error()?;
                    }
                }

                if let Some((cmds, persist)) = cmd.cmds_persist {
                    for bytecode in cmds.into_iter() {
                        let bytecode = bytecode.ok_or(Error::PacketParse(
                            crate::protocol::PacketParseError::MalformedCommand,
                        ))?;

                        let id = agent_ops.register_bytecode(bytecode).handle_error()?;
                        agent_ops
                            .add_breakpoint_bytecode(
                                BreakpointBytecodeKind::Command,
                                addr,
                                id,
                                persist,
                            )
                            .handle_error()?;
                    }
                }

                status
            }
            // The client sent a packet with bytecode even though the client never reported
            // implementing the `Agent` feature. This should never happen.
            _ => return Err(Error::PacketUnexpected),
        };
        Ok(handler_status)
    }
}
