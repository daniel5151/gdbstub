use armv4t_emu::{reg, Memory};

use gdbstub::common::{Signal, Tid};
use gdbstub::target;
use gdbstub::target::ext::base::multithread::{MultiThreadBase, MultiThreadResume};
use gdbstub::target::ext::breakpoints::WatchKind;
use gdbstub::target::{Target, TargetError, TargetResult};

use crate::emu::{CpuId, Emu, ExecMode};

pub fn cpuid_to_tid(id: CpuId) -> Tid {
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

    #[inline(always)]
    fn base_ops(&mut self) -> target::ext::base::BaseOps<Self::Arch, Self::Error> {
        target::ext::base::BaseOps::MultiThread(self)
    }

    #[inline(always)]
    fn support_breakpoints(&mut self) -> Option<target::ext::breakpoints::BreakpointsOps<Self>> {
        Some(self)
    }
}

impl MultiThreadBase for Emu {
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

    #[inline(always)]
    fn support_resume(
        &mut self,
    ) -> Option<target::ext::base::multithread::MultiThreadResumeOps<Self>> {
        Some(self)
    }
}

impl MultiThreadResume for Emu {
    fn resume(&mut self) -> Result<(), Self::Error> {
        // Upon returning from the `resume` method, the target being debugged should be
        // configured to run according to whatever resume actions the GDB client has
        // specified (as specified by `set_resume_action`, `set_resume_range_step`,
        // `set_reverse_{step, continue}`, etc...)
        //
        // In this basic `armv4t_multicore` example, the `resume` method is actually a
        // no-op, as the execution mode of the emulator's interpreter loop has already
        // been modified via the various `set_X` methods.
        //
        // In more complex implementations, it's likely that the target being debugged
        // will be running in another thread / process, and will require some kind of
        // external "orchestration" to set it's execution mode (e.g: modifying the
        // target's process state via platform specific debugging syscalls).

        Ok(())
    }

    fn clear_resume_actions(&mut self) -> Result<(), Self::Error> {
        self.exec_mode.clear();
        Ok(())
    }

    #[inline(always)]
    fn support_single_step(
        &mut self,
    ) -> Option<target::ext::base::multithread::MultiThreadSingleStepOps<Self>> {
        Some(self)
    }

    fn set_resume_action_continue(
        &mut self,
        tid: Tid,
        signal: Option<Signal>,
    ) -> Result<(), Self::Error> {
        if signal.is_some() {
            return Err("no support for continuing with signal");
        }

        self.exec_mode
            .insert(tid_to_cpuid(tid)?, ExecMode::Continue);

        Ok(())
    }
}

impl target::ext::base::multithread::MultiThreadSingleStep for Emu {
    fn set_resume_action_step(
        &mut self,
        tid: Tid,
        signal: Option<Signal>,
    ) -> Result<(), Self::Error> {
        if signal.is_some() {
            return Err("no support for stepping with signal");
        }

        self.exec_mode.insert(tid_to_cpuid(tid)?, ExecMode::Step);

        Ok(())
    }
}

impl target::ext::breakpoints::Breakpoints for Emu {
    fn support_sw_breakpoint(&mut self) -> Option<target::ext::breakpoints::SwBreakpointOps<Self>> {
        Some(self)
    }

    fn support_hw_watchpoint(&mut self) -> Option<target::ext::breakpoints::HwWatchpointOps<Self>> {
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
    fn add_hw_watchpoint(
        &mut self,
        addr: u32,
        _len: u32, // TODO: properly handle `len` parameter
        kind: WatchKind,
    ) -> TargetResult<bool, Self> {
        self.watchpoints.push(addr);

        let entry = self.watchpoint_kind.entry(addr).or_insert((false, false));
        match kind {
            WatchKind::Write => entry.1 = true,
            WatchKind::Read => entry.0 = true,
            WatchKind::ReadWrite => entry.0 = true, // arbitrary
        };

        Ok(true)
    }

    fn remove_hw_watchpoint(
        &mut self,
        addr: u32,
        _len: u32, // TODO: properly handle `len` parameter
        kind: WatchKind,
    ) -> TargetResult<bool, Self> {
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
