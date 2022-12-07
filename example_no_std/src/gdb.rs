use gdbstub::common::{Signal, Tid};
use gdbstub::target;
use gdbstub::target::ext::base::multithread::{MultiThreadBase, MultiThreadResume};
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

    #[inline(always)]
    fn base_ops(&mut self) -> target::ext::base::BaseOps<'_, Self::Arch, Self::Error> {
        target::ext::base::BaseOps::MultiThread(self)
    }

    // disable `QStartNoAckMode` in order to save space
    #[inline(always)]
    fn use_no_ack_mode(&self) -> bool {
        false
    }

    // disable X packet optimization in order to save space
    #[inline(always)]
    fn use_x_upcase_packet(&self) -> bool {
        false
    }

    #[inline(always)]
    fn support_breakpoints(
        &mut self,
    ) -> Option<target::ext::breakpoints::BreakpointsOps<'_, Self>> {
        Some(self)
    }
}

// NOTE: to try and make this a marginally more realistic estimate of
// `gdbstub`'s library overhead, non-IDET methods are marked as
// `#[inline(never)]` to prevent the optimizer from too aggressively coalescing
// the stubbed implementations.
//
// EXCEPTION: `list_active_threads` accepts a closure arg, and should be
// be inlined for smaller codegen

impl MultiThreadBase for DummyTarget {
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
    ) -> TargetResult<usize, Self> {
        print_str("> read_addrs");
        data.iter_mut().for_each(|b| *b = 0x55);
        Ok(data.len())
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

    #[inline(always)] // !! EXCEPTION !!
    fn list_active_threads(
        &mut self,
        register_thread: &mut dyn FnMut(Tid),
    ) -> Result<(), Self::Error> {
        print_str("> list_active_threads");
        register_thread(Tid::new(1).unwrap());
        register_thread(Tid::new(2).unwrap());
        Ok(())
    }

    #[inline(always)]
    fn support_resume(
        &mut self,
    ) -> Option<target::ext::base::multithread::MultiThreadResumeOps<'_, Self>> {
        Some(self)
    }
}

impl MultiThreadResume for DummyTarget {
    #[inline(never)]
    fn resume(&mut self) -> Result<(), Self::Error> {
        print_str("> resume");
        Ok(())
    }

    #[inline(never)]
    fn clear_resume_actions(&mut self) -> Result<(), Self::Error> {
        print_str("> clear_resume_actions");
        Ok(())
    }

    #[inline(never)]
    fn set_resume_action_continue(
        &mut self,
        _tid: Tid,
        _signal: Option<Signal>,
    ) -> Result<(), Self::Error> {
        print_str("> set_resume_action_continue");
        Ok(())
    }
}

impl target::ext::breakpoints::Breakpoints for DummyTarget {
    #[inline(always)]
    fn support_sw_breakpoint(
        &mut self,
    ) -> Option<target::ext::breakpoints::SwBreakpointOps<'_, Self>> {
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
