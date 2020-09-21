use armv4t_emu::{reg, Memory};

use gdbstub::arch;
use gdbstub::common::Tid;
use gdbstub::target::Target;
use gdbstub::target_ext;
use gdbstub::target_ext::base::multithread::{
    Actions, MultiThreadOps, ResumeAction, ThreadStopReason,
};
use gdbstub::target_ext::breakpoints::WatchKind;
use gdbstub::target_ext::monitor_cmd::{outputln, ConsoleOutput};

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
    type Arch = arch::arm::Armv4t;
    type Error = &'static str;

    fn base_ops(&mut self) -> target_ext::base::BaseOps<Self::Arch, Self::Error> {
        target_ext::base::BaseOps::MultiThread(self)
    }

    fn sw_breakpoint(&mut self) -> Option<target_ext::breakpoints::SwBreakpointOps<Self>> {
        Some(self)
    }

    fn hw_watchpoint(&mut self) -> Option<target_ext::breakpoints::HwWatchpointOps<Self>> {
        Some(self)
    }

    fn monitor_cmd(&mut self) -> Option<target_ext::monitor_cmd::MonitorCmdOps<Self>> {
        Some(self)
    }
}

impl MultiThreadOps for Emu {
    fn resume(
        &mut self,
        actions: Actions,
        check_gdb_interrupt: &mut dyn FnMut() -> bool,
    ) -> Result<ThreadStopReason<u32>, Self::Error> {
        // in this emulator, each core runs in lock-step, so we can ignore the
        // TidSelector associated with each action, and only care if GDB
        // requests execution to start / stop.
        //
        // In general, the behavior of multi-threaded systems during debugging is
        // determined by the system scheduler. On certain systems, this behavior can be
        // configured using the GDB command `set scheduler-locking _mode_`, but at the
        // moment, `gdbstub` doesn't plumb-through that configuration command.

        // FIXME: properly handle multiple actions...
        let actions = actions.collect::<Vec<_>>();
        let (_, action) = actions[0];

        match action {
            ResumeAction::Step => match self.step() {
                Some((event, id)) => Ok(event_to_stopreason(event, id)),
                None => Ok(ThreadStopReason::DoneStep),
            },
            ResumeAction::Continue => {
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

impl target_ext::breakpoints::SwBreakpoint for Emu {
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

impl target_ext::breakpoints::HwWatchpoint for Emu {
    fn add_hw_watchpoint(&mut self, addr: u32, kind: WatchKind) -> Result<bool, &'static str> {
        self.watchpoints.push(addr);

        let entry = self.watchpoint_kind.entry(addr).or_insert((false, false));
        match kind {
            WatchKind::Write => entry.1 = true,
            WatchKind::Read => entry.0 = true,
            WatchKind::ReadWrite => entry.0 = true, // arbitrary
        };

        Ok(true)
    }

    fn remove_hw_watchpoint(&mut self, addr: u32, kind: WatchKind) -> Result<bool, &'static str> {
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

impl target_ext::monitor_cmd::MonitorCmd for Emu {
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
