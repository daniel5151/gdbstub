use armv4t_emu::{reg, Memory};

use gdbstub::common::Tid;
use gdbstub::target;
use gdbstub::target::ext::base::multithread::{MultiThreadOps, ResumeAction, ThreadStopReason};
use gdbstub::target::ext::breakpoints::WatchKind;
use gdbstub::target::{Target, TargetError, TargetResult};

use crate::emu::{CpuId, Emu, Event};

fn event_to_stopreason(e: Event, id: CpuId) -> ThreadStopReason<u32> {
    let tid = cpuid_to_tid(id);
    match e {
        Event::Halted => ThreadStopReason::Halted,
        Event::Break => ThreadStopReason::SwBreak(tid),
        Event::WatchWrite(addr) => ThreadStopReason::Watch {
            tid,
            kind: WatchKind::Write,
            addr,
        },
        Event::WatchRead(addr) => ThreadStopReason::Watch {
            tid,
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
    type Arch = gdbstub_arch::arm::Armv4t;
    type Error = &'static str;

    fn base_ops(&mut self) -> target::ext::base::BaseOps<Self::Arch, Self::Error> {
        target::ext::base::BaseOps::MultiThread(self)
    }

    fn breakpoints(&mut self) -> Option<target::ext::breakpoints::BreakpointsOps<Self>> {
        Some(self)
    }
}

impl MultiThreadOps for Emu {
    fn resume(
        &mut self,
        default_resume_action: ResumeAction,
        check_gdb_interrupt: &mut dyn FnMut() -> bool,
    ) -> Result<ThreadStopReason<u32>, Self::Error> {
        // In general, the behavior of multi-threaded systems during debugging is
        // determined by the system scheduler. On certain systems, this behavior can be
        // configured using the GDB command `set scheduler-locking _mode_`, but at the
        // moment, `gdbstub` doesn't plumb-through that configuration command.

        let default_resume_action_is_step = match default_resume_action {
            ResumeAction::Step => true,
            ResumeAction::Continue => false,
            _ => return Err("no support for resuming with signal"),
        };

        match self
            .resume_action_is_step
            .unwrap_or(default_resume_action_is_step)
        {
            true => match self.step() {
                Some((event, id)) => Ok(event_to_stopreason(event, id)),
                None => Ok(ThreadStopReason::DoneStep),
            },
            false => {
                let mut cycles: usize = 0;
                loop {
                    // check for GDB interrupt every 1024 instructions
                    if cycles % 1024 == 0 && check_gdb_interrupt() {
                        return Ok(ThreadStopReason::GdbInterrupt);
                    }
                    cycles += 1;

                    if let Some((event, id)) = self.step() {
                        return Ok(event_to_stopreason(event, id));
                    };
                }
            }
        }
    }

    // FIXME: properly handle multiple actions
    fn clear_resume_actions(&mut self) -> Result<(), Self::Error> {
        self.resume_action_is_step = None;
        Ok(())
    }

    // FIXME: properly handle multiple actions
    fn set_resume_action(&mut self, _tid: Tid, action: ResumeAction) -> Result<(), Self::Error> {
        // in this emulator, each core runs in lock-step, so we don't actually care
        // about the specific tid. In real integrations, you very much should!

        if self.resume_action_is_step.is_some() {
            return Ok(());
        }

        self.resume_action_is_step = match action {
            ResumeAction::Step => Some(true),
            ResumeAction::Continue => Some(false),
            _ => return Err("no support for resuming with signal"),
        };

        Ok(())
    }

    fn read_registers(
        &mut self,
        regs: &mut gdbstub_arch::arm::reg::ArmCoreRegs,
        tid: Tid,
    ) -> TargetResult<(), Self> {
        let cpu = match tid_to_cpuid(tid).map_err(TargetError::Fatal)? {
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
        regs: &gdbstub_arch::arm::reg::ArmCoreRegs,
        tid: Tid,
    ) -> TargetResult<(), Self> {
        let cpu = match tid_to_cpuid(tid).map_err(TargetError::Fatal)? {
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
    ) -> TargetResult<(), Self> {
        for (addr, val) in (start_addr..).zip(data.iter_mut()) {
            *val = self.mem.r8(addr)
        }
        Ok(())
    }

    fn write_addrs(
        &mut self,
        start_addr: u32,
        data: &[u8],
        _tid: Tid, // same address space for each core
    ) -> TargetResult<(), Self> {
        for (addr, val) in (start_addr..).zip(data.iter().copied()) {
            self.mem.w8(addr, val)
        }
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
}

impl target::ext::breakpoints::Breakpoints for Emu {
    fn sw_breakpoint(&mut self) -> Option<target::ext::breakpoints::SwBreakpointOps<Self>> {
        Some(self)
    }

    fn hw_watchpoint(&mut self) -> Option<target::ext::breakpoints::HwWatchpointOps<Self>> {
        Some(self)
    }
}

impl target::ext::breakpoints::SwBreakpoint for Emu {
    fn add_sw_breakpoint(
        &mut self,
        addr: u32,
        _kind: gdbstub_arch::arm::ArmBreakpointKind,
    ) -> TargetResult<bool, Self> {
        self.breakpoints.push(addr);
        Ok(true)
    }

    fn remove_sw_breakpoint(
        &mut self,
        addr: u32,
        _kind: gdbstub_arch::arm::ArmBreakpointKind,
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
        self.watchpoints.push(addr);

        let entry = self.watchpoint_kind.entry(addr).or_insert((false, false));
        match kind {
            WatchKind::Write => entry.1 = true,
            WatchKind::Read => entry.0 = true,
            WatchKind::ReadWrite => entry.0 = true, // arbitrary
        };

        Ok(true)
    }

    fn remove_hw_watchpoint(&mut self, addr: u32, kind: WatchKind) -> TargetResult<bool, Self> {
        let entry = self.watchpoint_kind.entry(addr).or_insert((false, false));
        match kind {
            WatchKind::Write => entry.1 = false,
            WatchKind::Read => entry.0 = false,
            WatchKind::ReadWrite => entry.0 = false, // arbitrary
        };

        if !self.watchpoint_kind.contains_key(&addr) {
            let pos = match self.watchpoints.iter().position(|x| *x == addr) {
                None => return Ok(false),
                Some(pos) => pos,
            };
            self.watchpoints.remove(pos);
        }

        Ok(true)
    }
}
