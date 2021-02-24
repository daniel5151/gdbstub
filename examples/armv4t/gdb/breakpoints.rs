use gdbstub::arch;
use gdbstub::target;
use gdbstub::target::ext::agent::BytecodeId;
use gdbstub::target::ext::breakpoints::{BreakpointAgentOps, BreakpointBytecodeKind, WatchKind};
use gdbstub::target::TargetResult;

use crate::emu::Emu;

impl target::ext::breakpoints::Breakpoints for Emu {
    fn sw_breakpoint(&mut self) -> Option<target::ext::breakpoints::SwBreakpointOps<Self>> {
        Some(self)
    }

    fn hw_watchpoint(&mut self) -> Option<target::ext::breakpoints::HwWatchpointOps<Self>> {
        Some(self)
    }

    fn breakpoint_agent(&mut self) -> Option<BreakpointAgentOps<Self>> {
        Some(self)
    }
}

impl target::ext::breakpoints::SwBreakpoint for Emu {
    fn add_sw_breakpoint(
        &mut self,
        addr: u32,
        _kind: arch::arm::ArmBreakpointKind,
    ) -> TargetResult<bool, Self> {
        self.breakpoints.push(addr);
        Ok(true)
    }

    fn remove_sw_breakpoint(
        &mut self,
        addr: u32,
        _kind: arch::arm::ArmBreakpointKind,
    ) -> TargetResult<bool, Self> {
        match self.breakpoints.iter().position(|x| *x == addr) {
            None => return Ok(false),
            Some(pos) => self.breakpoints.remove(pos),
        };

        Ok(true)
    }
}

impl target::ext::breakpoints::HwWatchpoint for Emu {
    fn add_hw_watchpoint(&mut self, addr: u32, kind: WatchKind) -> TargetResult<bool, Self> {
        match kind {
            WatchKind::Write => self.watchpoints.push(addr),
            WatchKind::Read => self.watchpoints.push(addr),
            WatchKind::ReadWrite => self.watchpoints.push(addr),
        };

        Ok(true)
    }

    fn remove_hw_watchpoint(&mut self, addr: u32, kind: WatchKind) -> TargetResult<bool, Self> {
        let pos = match self.watchpoints.iter().position(|x| *x == addr) {
            None => return Ok(false),
            Some(pos) => pos,
        };

        match kind {
            WatchKind::Write => self.watchpoints.remove(pos),
            WatchKind::Read => self.watchpoints.remove(pos),
            WatchKind::ReadWrite => self.watchpoints.remove(pos),
        };

        Ok(true)
    }
}

impl target::ext::breakpoints::BreakpointAgent for Emu {
    fn add_breakpoint_bytecode(
        &mut self,
        kind: BreakpointBytecodeKind,
        addr: u32,
        id: BytecodeId,
        _persist: bool,
    ) -> TargetResult<(), Self> {
        log::warn!("Registered {:?} {:#010x?}:{:?}", kind, addr, id);

        match kind {
            BreakpointBytecodeKind::Command => &mut self.agent.breakpoint_commands,
            BreakpointBytecodeKind::Condition => &mut self.agent.breakpoint_conditions,
        }
        .as_mut()
        .unwrap()
        .entry(addr)
        .or_default()
        .push(id);

        Ok(())
    }

    fn clear_breakpoint_bytecode(
        &mut self,
        kind: BreakpointBytecodeKind,
        addr: u32,
    ) -> TargetResult<(), Self> {
        log::warn!("Unregistered all {:?} from {:#010x?}", kind, addr);

        if let Some(s) = match kind {
            BreakpointBytecodeKind::Command => &mut self.agent.breakpoint_commands,
            BreakpointBytecodeKind::Condition => &mut self.agent.breakpoint_conditions,
        }
        .as_mut()
        .unwrap()
        .get_mut(&addr)
        {
            s.clear()
        }

        Ok(())
    }

    fn get_breakpoint_bytecode(
        &mut self,
        kind: BreakpointBytecodeKind,
        addr: u32,
        callback: &mut dyn FnMut(BreakpointAgentOps<Self>, BytecodeId) -> Result<(), Self::Error>,
    ) -> Result<(), Self::Error> {
        log::warn!(
            "Iterating over all {:?} bytecode expressions at {:#010x?}",
            kind,
            addr
        );

        // See this method's documentation for why this additional `Option::take`
        // rigamarole is required

        let mut cmds = match kind {
            BreakpointBytecodeKind::Command => &mut self.agent.breakpoint_commands,
            BreakpointBytecodeKind::Condition => &mut self.agent.breakpoint_conditions,
        }
        .take()
        .unwrap();

        let ids = cmds.entry(addr).or_default();

        let mut res = Ok(());
        for id in ids.iter() {
            res = callback(self, *id);
            if res.is_err() {
                break;
            }
        }

        match kind {
            BreakpointBytecodeKind::Command => self.agent.breakpoint_commands = Some(cmds),
            BreakpointBytecodeKind::Condition => self.agent.breakpoint_conditions = Some(cmds),
        };

        res
    }
}
