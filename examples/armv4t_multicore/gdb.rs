use armv4t_emu::{reg, Memory};
use gdbstub::{
    arch, outputln, Actions, BreakOp, ConsoleOutput, OptResult, ResumeAction, StopReason, Target,
    Tid, WatchKind,
};

use crate::emu::{CpuId, Emu, Event};

fn event_to_stopreason(e: Event) -> StopReason<u32> {
    match e {
        Event::Halted => StopReason::Halted,
        Event::Break => StopReason::HwBreak,
        Event::WatchWrite(addr) => StopReason::Watch {
            kind: WatchKind::Write,
            addr,
        },
        Event::WatchRead(addr) => StopReason::Watch {
            kind: WatchKind::Read,
            addr,
        },
    }
}

fn cpuid_to_tid(id: CpuId) -> Tid {
    match id {
        CpuId::Cpu => Tid::new(1).unwrap(),
        CpuId::Cop => Tid::new(2).unwrap(),
    }
}

impl Target for Emu {
    type Arch = arch::arm::Armv4t;
    type Error = &'static str;

    fn resume(
        &mut self,
        actions: Actions,
        check_gdb_interrupt: &mut dyn FnMut() -> bool,
    ) -> Result<(Tid, StopReason<u32>), Self::Error> {
        // in this emulator, we ignore the Tid associated with the action, and only care
        // if GDB requests execution to start / stop. Each core runs in lock-step.
        //
        // In general, the behavior of multi-threaded systems during debugging is
        // determined by the system scheduler. On certain systems, this behavior can be
        // configured using the GDB command `set scheduler-locking _mode_`, but at the
        // moment, `gdbstub` doesn't plumb-through that option.

        let actions = actions.collect::<Vec<_>>();
        if actions.len() != 1 {
            // AFAIK, this will never happen on such a simple system. Plus, it's just an
            // example, cut me some slack!
            return Err("too lazy to implement support for more than one action :P");
        }
        let action = actions[0].1;

        match action {
            ResumeAction::Step => match self.step() {
                Some((event, id)) => Ok((cpuid_to_tid(id), event_to_stopreason(event))),
                None => Ok((cpuid_to_tid(self.selected_core), StopReason::DoneStep)),
            },
            ResumeAction::Continue => {
                let mut cycles: usize = 0;
                loop {
                    // check for GDB interrupt every 1024 instructions
                    if cycles % 1024 == 0 && check_gdb_interrupt() {
                        return Ok((cpuid_to_tid(self.selected_core), StopReason::GdbInterrupt));
                    }
                    cycles += 1;

                    if let Some((event, id)) = self.step() {
                        return Ok((cpuid_to_tid(id), event_to_stopreason(event)));
                    };
                }
            }
        }
    }

    fn read_registers(
        &mut self,
        regs: &mut arch::arm::reg::ArmCoreRegs,
    ) -> Result<(), &'static str> {
        let cpu = match self.selected_core {
            CpuId::Cpu => &mut self.cpu,
            CpuId::Cop => &mut self.cop,
        };

        let mode = cpu.mode();

        for i in 0..13 {
            regs.r[i] = cpu.reg_get(mode, i as u8);
        }
        regs.sp = cpu.reg_get(mode, reg::SP);
        regs.lr = cpu.reg_get(mode, reg::LR);
        regs.pc = cpu.reg_get(mode, reg::PC);
        regs.cpsr = cpu.reg_get(mode, reg::CPSR);

        Ok(())
    }

    fn write_registers(&mut self, regs: &arch::arm::reg::ArmCoreRegs) -> Result<(), &'static str> {
        let cpu = match self.selected_core {
            CpuId::Cpu => &mut self.cpu,
            CpuId::Cop => &mut self.cop,
        };

        let mode = cpu.mode();

        for i in 0..13 {
            cpu.reg_set(mode, i, regs.r[i as usize]);
        }
        cpu.reg_set(mode, reg::SP, regs.sp);
        cpu.reg_set(mode, reg::LR, regs.lr);
        cpu.reg_set(mode, reg::PC, regs.pc);
        cpu.reg_set(mode, reg::CPSR, regs.cpsr);

        Ok(())
    }

    fn read_addrs(&mut self, start_addr: u32, data: &mut [u8]) -> Result<bool, &'static str> {
        for (addr, val) in (start_addr..).zip(data.iter_mut()) {
            *val = self.mem.r8(addr)
        }
        Ok(true)
    }

    fn write_addrs(&mut self, start_addr: u32, data: &[u8]) -> Result<bool, &'static str> {
        for (addr, val) in (start_addr..).zip(data.iter().copied()) {
            self.mem.w8(addr, val)
        }
        Ok(true)
    }

    fn update_sw_breakpoint(&mut self, addr: u32, op: BreakOp) -> Result<bool, &'static str> {
        match op {
            BreakOp::Add => self.breakpoints.push(addr),
            BreakOp::Remove => {
                let pos = match self.breakpoints.iter().position(|x| *x == addr) {
                    None => return Ok(false),
                    Some(pos) => pos,
                };
                self.breakpoints.remove(pos);
            }
        }

        Ok(true)
    }

    fn update_hw_watchpoint(
        &mut self,
        addr: u32,
        op: BreakOp,
        kind: WatchKind,
    ) -> OptResult<bool, &'static str> {
        match op {
            BreakOp::Add => {
                match kind {
                    WatchKind::Write => self.watchpoints.push(addr),
                    WatchKind::Read => self.watchpoints.push(addr),
                    WatchKind::ReadWrite => self.watchpoints.push(addr),
                };
            }
            BreakOp::Remove => {
                let pos = match self.watchpoints.iter().position(|x| *x == addr) {
                    None => return Ok(false),
                    Some(pos) => pos,
                };

                match kind {
                    WatchKind::Write => self.watchpoints.remove(pos),
                    WatchKind::Read => self.watchpoints.remove(pos),
                    WatchKind::ReadWrite => self.watchpoints.remove(pos),
                };
            }
        }

        Ok(true)
    }

    fn handle_monitor_cmd(
        &mut self,
        cmd: &[u8],
        mut out: ConsoleOutput<'_>,
    ) -> OptResult<(), Self::Error> {
        let cmd = match core::str::from_utf8(cmd) {
            Ok(cmd) => cmd,
            Err(_) => {
                outputln!(out, "command must be valid UTF-8");
                return Ok(());
            }
        };

        match cmd {
            "" => outputln!(out, "Sorry, didn't catch that. Try `monitor ping`!"),
            "ping" => outputln!(out, "pong!"),
            _ => outputln!(out, "I don't know how to handle '{}'", cmd),
        };

        Ok(())
    }

    fn list_active_threads(
        &mut self,
        register_thread: &mut dyn FnMut(Tid),
    ) -> Result<(), Self::Error> {
        register_thread(cpuid_to_tid(CpuId::Cpu));
        register_thread(cpuid_to_tid(CpuId::Cop));
        Ok(())
    }

    fn set_current_thread(&mut self, tid: Tid) -> OptResult<(), Self::Error> {
        match tid.get() {
            1 => self.selected_core = CpuId::Cpu,
            2 => self.selected_core = CpuId::Cop,
            _ => return Err("specified invalid core".into()),
        }
        Ok(())
    }
}
