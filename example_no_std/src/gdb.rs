use gdbstub::arch;
use gdbstub::target::base::multithread::{Actions, MultiThreadOps, ThreadStopReason, Tid};
use gdbstub::target::ext::breakpoint::WatchKind;
use gdbstub::target::ext::monitor::ConsoleOutput;
use gdbstub::target::{base, ext, Target};

use crate::print_str::print_str;

pub struct DummyTarget {}

impl DummyTarget {
    pub fn new() -> DummyTarget {
        DummyTarget {}
    }
}

impl Target for DummyTarget {
    type Arch = arch::arm::Armv4t;
    type Error = &'static str;

    fn base_ops(&mut self) -> base::BaseOps<Self::Arch, Self::Error> {
        base::BaseOps::MultiThread(self)
    }

    fn sw_breakpoint(&mut self) -> ext::SwBreakpointExt<Self> {
        self
    }

    fn monitor_cmd(&mut self) -> Option<ext::MonitorCmdExt<Self>> {
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
        _actions: Actions,
        _check_gdb_interrupt: &mut dyn FnMut() -> bool,
    ) -> Result<ThreadStopReason<u32>, Self::Error> {
        print_str("> resume");
        Ok(ThreadStopReason::DoneStep)
    }

    #[inline(never)]
    fn read_registers(
        &mut self,
        _regs: &mut arch::arm::reg::ArmCoreRegs,
        _tid: Tid,
    ) -> Result<(), &'static str> {
        print_str("> read_registers");
        Ok(())
    }

    #[inline(never)]
    fn write_registers(
        &mut self,
        _regs: &arch::arm::reg::ArmCoreRegs,
        _tid: Tid,
    ) -> Result<(), &'static str> {
        print_str("> write_registers");
        Ok(())
    }

    #[inline(never)]
    fn read_addrs(
        &mut self,
        _start_addr: u32,
        data: &mut [u8],
        _tid: Tid, // same address space for each core
    ) -> Result<bool, &'static str> {
        print_str("> read_addrs");
        data.iter_mut().for_each(|b| *b = 0x55);
        Ok(true)
    }

    #[inline(never)]
    fn write_addrs(
        &mut self,
        _start_addr: u32,
        _data: &[u8],
        _tid: Tid, // same address space for each core
    ) -> Result<bool, &'static str> {
        print_str("> write_addrs");
        Ok(true)
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

impl ext::breakpoint::SwBreakpoint for DummyTarget {
    #[inline(never)]
    fn add_sw_breakpoint(&mut self, _addr: u32) -> Result<bool, &'static str> {
        Ok(true)
    }

    #[inline(never)]
    fn remove_sw_breakpoint(&mut self, _addr: u32) -> Result<bool, &'static str> {
        Ok(true)
    }
}

impl ext::breakpoint::HwWatchpoint for DummyTarget {
    #[inline(never)]
    fn add_hw_watchpoint(&mut self, _addr: u32, _kind: WatchKind) -> Result<bool, &'static str> {
        Ok(true)
    }

    #[inline(never)]
    fn remove_hw_watchpoint(&mut self, _addr: u32, _kind: WatchKind) -> Result<bool, &'static str> {
        Ok(true)
    }
}

impl ext::monitor::MonitorCmd for DummyTarget {
    #[inline(never)]
    fn handle_monitor_cmd(
        &mut self,
        cmd: &[u8],
        mut out: ConsoleOutput<'_>,
    ) -> Result<(), Self::Error> {
        match cmd {
            b"" => out.write_raw(b"Sorry, didn't catch that. Try `monitor ping`!\n"),
            b"ping" => out.write_raw(b"pong!\n"),
            _ => out.write_raw(b"I don't know how to handle that\n"),
        };

        Ok(())
    }
}
