use armv4t_emu::{reg, Memory};
use gdbstub::arch;
use gdbstub::target::base::{Actions, ResumeAction, StopReason, Tid};
use gdbstub::target::ext::breakpoint::WatchKind;
use gdbstub::target::ext::monitor::{outputln, ConsoleOutput};
use gdbstub::target::{base, ext, Target};

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

fn tid_to_cpuid(tid: Tid) -> Result<CpuId, &'static str> {
    match tid.get() {
        1 => Ok(CpuId::Cpu),
        2 => Ok(CpuId::Cop),
        _ => Err("specified invalid core"),
    }
}

impl Target for Emu {
    type Arch = arch::arm::Armv4t;
    type Error = &'static str;

    fn base_ops(&mut self) -> base::BaseOps<Self::Arch, Self::Error> {
        base::BaseOps::MultiThread(self)
    }

    fn sw_breakpoint(&mut self) -> ext::SwBreakpointExt<Self> {
        self
    }

    fn hw_watchpoint(&mut self) -> Option<ext::HwWatchpointExt<Self>> {
        Some(self)
    }

    fn monitor_cmd(&mut self) -> Option<ext::MonitorCmdExt<Self>> {
        Some(self)
    }
}

impl base::MultiThread for Emu {
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
        let (tid_selector, action) = actions[0];

        let tid = match tid_selector {
            base::TidSelector::WithID(id) => id,
            _ => cpuid_to_tid(CpuId::Cpu), // ...
        };

        match action {
            ResumeAction::Step => match self.step() {
                Some((event, id)) => Ok((cpuid_to_tid(id), event_to_stopreason(event))),
                None => Ok((tid, StopReason::DoneStep)),
            },
            ResumeAction::Continue => {
                let mut cycles: usize = 0;
                loop {
                    // check for GDB interrupt every 1024 instructions
                    if cycles % 1024 == 0 && check_gdb_interrupt() {
                        return Ok((tid, StopReason::GdbInterrupt));
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
        tid: Tid,
    ) -> Result<(), &'static str> {
        let cpu = match tid_to_cpuid(tid)? {
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

    fn write_registers(
        &mut self,
        regs: &arch::arm::reg::ArmCoreRegs,
        tid: Tid,
    ) -> Result<(), &'static str> {
        let cpu = match tid_to_cpuid(tid)? {
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

    fn read_addrs(
        &mut self,
        start_addr: u32,
        data: &mut [u8],
        _tid: Tid, // same address space for each core
    ) -> Result<bool, &'static str> {
        for (addr, val) in (start_addr..).zip(data.iter_mut()) {
            *val = self.mem.r8(addr)
        }
        Ok(true)
    }

    fn write_addrs(
        &mut self,
        start_addr: u32,
        data: &[u8],
        _tid: Tid, // same address space for each core
    ) -> Result<bool, &'static str> {
        for (addr, val) in (start_addr..).zip(data.iter().copied()) {
            self.mem.w8(addr, val)
        }
        Ok(true)
    }

    fn list_active_threads(
        &mut self,
        register_thread: &mut dyn FnMut(Tid),
    ) -> Result<(), Self::Error> {
        register_thread(cpuid_to_tid(CpuId::Cpu));
        register_thread(cpuid_to_tid(CpuId::Cop));
        Ok(())
    }
}

impl ext::breakpoint::SwBreakpoint for Emu {
    fn add_sw_breakpoint(&mut self, addr: u32) -> Result<bool, &'static str> {
        self.breakpoints.push(addr);
        Ok(true)
    }

    fn remove_sw_breakpoint(&mut self, addr: u32) -> Result<bool, &'static str> {
        match self.breakpoints.iter().position(|x| *x == addr) {
            None => return Ok(false),
            Some(pos) => self.breakpoints.remove(pos),
        };

        Ok(true)
    }
}

impl ext::breakpoint::HwWatchpoint for Emu {
    fn add_hw_watchpoint(&mut self, addr: u32, kind: WatchKind) -> Result<bool, &'static str> {
        match kind {
            WatchKind::Write => self.watchpoints.push(addr),
            WatchKind::Read => self.watchpoints.push(addr),
            WatchKind::ReadWrite => self.watchpoints.push(addr),
        };

        Ok(true)
    }

    fn remove_hw_watchpoint(&mut self, addr: u32, kind: WatchKind) -> Result<bool, &'static str> {
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

impl ext::monitor::MonitorCmd for Emu {
    fn handle_monitor_cmd(
        &mut self,
        cmd: &[u8],
        mut out: ConsoleOutput<'_>,
    ) -> Result<(), Self::Error> {
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
}
