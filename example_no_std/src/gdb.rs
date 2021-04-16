use gdbstub::common::Tid;
use gdbstub::target;
use gdbstub::target::ext::base::multithread::{MultiThreadOps, ResumeAction, ThreadStopReason};
use gdbstub::target::{Target, TargetResult};

use crate::print_str::print_str;

pub struct DummyTarget {}

impl DummyTarget {
    pub fn new() -> DummyTarget {
        DummyTarget {}
    }
}

impl Target for DummyTarget {
    type Arch = gdbstub_arch::arm::Armv4t;
    type Error = &'static str;

    fn base_ops(&mut self) -> target::ext::base::BaseOps<Self::Arch, Self::Error> {
        target::ext::base::BaseOps::MultiThread(self)
    }

    fn breakpoints(&mut self) -> Option<target::ext::breakpoints::BreakpointsOps<Self>> {
        Some(self)
    }
}

// NOTE: to try and make this a more realistic example, methods are marked as
// `#[inline(never)]` to prevent the optimizer from too aggressively coalescing
// the stubbed implementations.

impl MultiThreadOps for DummyTarget {
    #[inline(never)]
    fn resume(
        &mut self,
        _default_resume_action: ResumeAction,
        _check_gdb_interrupt: &mut dyn FnMut() -> bool,
    ) -> Result<ThreadStopReason<u32>, Self::Error> {
        print_str("> resume");
        Ok(ThreadStopReason::DoneStep)
    }

    #[inline(never)]
    fn clear_resume_actions(&mut self) -> Result<(), Self::Error> {
        print_str("> clear_resume_actions");
        Ok(())
    }

    #[inline(never)]
    fn set_resume_action(&mut self, _tid: Tid, _action: ResumeAction) -> Result<(), Self::Error> {
        print_str("> set_resume_action");
        Ok(())
    }

    #[inline(never)]
    fn read_registers(
        &mut self,
        _regs: &mut gdbstub_arch::arm::reg::ArmCoreRegs,
        _tid: Tid,
    ) -> TargetResult<(), Self> {
        print_str("> read_registers");
        Ok(())
    }

    #[inline(never)]
    fn write_registers(
        &mut self,
        _regs: &gdbstub_arch::arm::reg::ArmCoreRegs,
        _tid: Tid,
    ) -> TargetResult<(), Self> {
        print_str("> write_registers");
        Ok(())
    }

    #[inline(never)]
    fn read_addrs(
        &mut self,
        _start_addr: u32,
        data: &mut [u8],
        _tid: Tid, // same address space for each core
    ) -> TargetResult<(), Self> {
        print_str("> read_addrs");
        data.iter_mut().for_each(|b| *b = 0x55);
        Ok(())
    }

    #[inline(never)]
    fn write_addrs(
        &mut self,
        _start_addr: u32,
        _data: &[u8],
        _tid: Tid, // same address space for each core
    ) -> TargetResult<(), Self> {
        print_str("> write_addrs");
        Ok(())
    }

    #[inline(never)]
    fn list_active_threads(
        &mut self,
        register_thread: &mut dyn FnMut(Tid),
    ) -> Result<(), Self::Error> {
        print_str("> list_active_threads");
        register_thread(Tid::new(1).unwrap());
        register_thread(Tid::new(2).unwrap());
        Ok(())
    }
}

impl target::ext::breakpoints::Breakpoints for DummyTarget {
    fn sw_breakpoint(&mut self) -> Option<target::ext::breakpoints::SwBreakpointOps<Self>> {
        Some(self)
    }
}

impl target::ext::breakpoints::SwBreakpoint for DummyTarget {
    #[inline(never)]
    fn add_sw_breakpoint(
        &mut self,
        _addr: u32,
        _kind: gdbstub_arch::arm::ArmBreakpointKind,
    ) -> TargetResult<bool, Self> {
        Ok(true)
    }

    #[inline(never)]
    fn remove_sw_breakpoint(
        &mut self,
        _addr: u32,
        _kind: gdbstub_arch::arm::ArmBreakpointKind,
    ) -> TargetResult<bool, Self> {
        Ok(true)
    }
}
