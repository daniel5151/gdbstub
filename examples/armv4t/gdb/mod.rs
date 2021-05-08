use core::convert::TryInto;

use armv4t_emu::{reg, Memory};
use gdbstub::target;
use gdbstub::target::ext::base::singlethread::{
    GdbInterrupt, ResumeAction, SingleThreadOps, SingleThreadReverseContOps,
    SingleThreadReverseStepOps, StopReason,
};
use gdbstub::target::ext::breakpoints::WatchKind;
use gdbstub::target::{Target, TargetError, TargetResult};
use gdbstub_arch::arm::reg::id::ArmCoreRegId;

use crate::emu::{Emu, Event};

// Additional GDB extensions

mod breakpoints;
mod extended_mode;
mod monitor_cmd;
mod section_offsets;
mod target_description_xml_override;

/// Turn a `ArmCoreRegId` into an internal register number of `armv4t_emu`.
fn cpu_reg_id(id: ArmCoreRegId) -> Option<u8> {
    match id {
        ArmCoreRegId::Gpr(i) => Some(i),
        ArmCoreRegId::Sp => Some(reg::SP),
        ArmCoreRegId::Lr => Some(reg::LR),
        ArmCoreRegId::Pc => Some(reg::PC),
        ArmCoreRegId::Cpsr => Some(reg::CPSR),
        _ => None,
    }
}

impl Target for Emu {
    type Arch = gdbstub_arch::arm::Armv4t;
    type Error = &'static str;

    // --------------- IMPORTANT NOTE ---------------
    // Always remember to annotate IDET enable methods with `inline(always)`!
    // Without this annotation, LLVM might fail to dead-code-eliminate nested IDET
    // implementations, resulting in unnecessary binary bloat.

    #[inline(always)]
    fn base_ops(&mut self) -> target::ext::base::BaseOps<Self::Arch, Self::Error> {
        target::ext::base::BaseOps::SingleThread(self)
    }

    #[inline(always)]
    fn breakpoints(&mut self) -> Option<target::ext::breakpoints::BreakpointsOps<Self>> {
        Some(self)
    }

    #[inline(always)]
    fn extended_mode(&mut self) -> Option<target::ext::extended_mode::ExtendedModeOps<Self>> {
        Some(self)
    }

    #[inline(always)]
    fn monitor_cmd(&mut self) -> Option<target::ext::monitor_cmd::MonitorCmdOps<Self>> {
        Some(self)
    }

    #[inline(always)]
    fn section_offsets(&mut self) -> Option<target::ext::section_offsets::SectionOffsetsOps<Self>> {
        Some(self)
    }

    #[inline(always)]
    fn target_description_xml_override(
        &mut self,
    ) -> Option<target::ext::target_description_xml_override::TargetDescriptionXmlOverrideOps<Self>>
    {
        Some(self)
    }
}

impl Emu {
    fn inner_resume(
        &mut self,
        action: ResumeAction,
        mut check_gdb_interrupt: impl FnMut() -> bool,
    ) -> Result<StopReason<u32>, &'static str> {
        let event = match action {
            ResumeAction::Step => match self.step() {
                Some(e) => e,
                None => return Ok(StopReason::DoneStep),
            },
            ResumeAction::Continue => {
                let mut cycles = 0;
                loop {
                    if let Some(event) = self.step() {
                        break event;
                    };

                    // check for GDB interrupt every 1024 instructions
                    cycles += 1;
                    if cycles % 1024 == 0 && check_gdb_interrupt() {
                        return Ok(StopReason::GdbInterrupt);
                    }
                }
            }
            _ => return Err("cannot resume with signal"),
        };

        Ok(match event {
            Event::Halted => StopReason::Terminated(19), // SIGSTOP
            Event::Break => StopReason::SwBreak,
            Event::WatchWrite(addr) => StopReason::Watch {
                kind: WatchKind::Write,
                addr,
            },
            Event::WatchRead(addr) => StopReason::Watch {
                kind: WatchKind::Read,
                addr,
            },
        })
    }
}

impl SingleThreadOps for Emu {
    fn resume(
        &mut self,
        action: ResumeAction,
        gdb_interrupt: GdbInterrupt<'_>,
    ) -> Result<StopReason<u32>, Self::Error> {
        let mut gdb_interrupt = gdb_interrupt.no_async();
        self.inner_resume(action, || gdb_interrupt.pending())
    }

    fn read_registers(
        &mut self,
        regs: &mut gdbstub_arch::arm::reg::ArmCoreRegs,
    ) -> TargetResult<(), Self> {
        let mode = self.cpu.mode();

        for i in 0..13 {
            regs.r[i] = self.cpu.reg_get(mode, i as u8);
        }
        regs.sp = self.cpu.reg_get(mode, reg::SP);
        regs.lr = self.cpu.reg_get(mode, reg::LR);
        regs.pc = self.cpu.reg_get(mode, reg::PC);
        regs.cpsr = self.cpu.reg_get(mode, reg::CPSR);

        Ok(())
    }

    fn write_registers(
        &mut self,
        regs: &gdbstub_arch::arm::reg::ArmCoreRegs,
    ) -> TargetResult<(), Self> {
        let mode = self.cpu.mode();

        for i in 0..13 {
            self.cpu.reg_set(mode, i, regs.r[i as usize]);
        }
        self.cpu.reg_set(mode, reg::SP, regs.sp);
        self.cpu.reg_set(mode, reg::LR, regs.lr);
        self.cpu.reg_set(mode, reg::PC, regs.pc);
        self.cpu.reg_set(mode, reg::CPSR, regs.cpsr);

        Ok(())
    }

    fn read_addrs(&mut self, start_addr: u32, data: &mut [u8]) -> TargetResult<(), Self> {
        for (addr, val) in (start_addr..).zip(data.iter_mut()) {
            *val = self.mem.r8(addr)
        }
        Ok(())
    }

    fn write_addrs(&mut self, start_addr: u32, data: &[u8]) -> TargetResult<(), Self> {
        for (addr, val) in (start_addr..).zip(data.iter().copied()) {
            self.mem.w8(addr, val)
        }
        Ok(())
    }

    #[inline(always)]
    fn single_register_access(
        &mut self,
    ) -> Option<target::ext::base::SingleRegisterAccessOps<(), Self>> {
        Some(self)
    }

    #[inline(always)]
    fn support_reverse_cont(&mut self) -> Option<SingleThreadReverseContOps<Self>> {
        Some(self)
    }

    #[inline(always)]
    fn support_reverse_step(&mut self) -> Option<SingleThreadReverseStepOps<Self>> {
        Some(self)
    }

    #[inline(always)]
    fn support_resume_range_step(
        &mut self,
    ) -> Option<target::ext::base::singlethread::SingleThreadRangeSteppingOps<Self>> {
        Some(self)
    }
}

impl target::ext::base::SingleRegisterAccess<()> for Emu {
    fn read_register(
        &mut self,
        _tid: (),
        reg_id: gdbstub_arch::arm::reg::id::ArmCoreRegId,
        dst: &mut [u8],
    ) -> TargetResult<(), Self> {
        if let Some(i) = cpu_reg_id(reg_id) {
            let w = self.cpu.reg_get(self.cpu.mode(), i);
            dst.copy_from_slice(&w.to_le_bytes());
            Ok(())
        } else {
            Err(().into())
        }
    }

    fn write_register(
        &mut self,
        _tid: (),
        reg_id: gdbstub_arch::arm::reg::id::ArmCoreRegId,
        val: &[u8],
    ) -> TargetResult<(), Self> {
        let w = u32::from_le_bytes(
            val.try_into()
                .map_err(|_| TargetError::Fatal("invalid data"))?,
        );
        if let Some(i) = cpu_reg_id(reg_id) {
            self.cpu.reg_set(self.cpu.mode(), i, w);
            Ok(())
        } else {
            Err(().into())
        }
    }
}

impl target::ext::base::singlethread::SingleThreadReverseCont for Emu {
    fn reverse_cont(
        &mut self,
        gdb_interrupt: GdbInterrupt<'_>,
    ) -> Result<StopReason<u32>, Self::Error> {
        // FIXME: actually implement reverse step
        eprintln!(
            "FIXME: Not actually reverse-continuing. Performing forwards continue instead..."
        );
        self.resume(ResumeAction::Continue, gdb_interrupt)
    }
}

impl target::ext::base::singlethread::SingleThreadReverseStep for Emu {
    fn reverse_step(
        &mut self,
        gdb_interrupt: GdbInterrupt<'_>,
    ) -> Result<StopReason<u32>, Self::Error> {
        // FIXME: actually implement reverse step
        eprintln!(
            "FIXME: Not actually reverse-stepping. Performing single forwards step instead..."
        );
        self.resume(ResumeAction::Step, gdb_interrupt)
    }
}

impl target::ext::base::singlethread::SingleThreadRangeStepping for Emu {
    fn resume_range_step(
        &mut self,
        start: u32,
        end: u32,
        gdb_interrupt: GdbInterrupt<'_>,
    ) -> Result<StopReason<u32>, Self::Error> {
        let mut gdb_interrupt = gdb_interrupt.no_async();
        loop {
            match self.inner_resume(ResumeAction::Step, || gdb_interrupt.pending())? {
                StopReason::DoneStep => {}
                stop_reason => return Ok(stop_reason),
            }

            if !(start..end).contains(&self.cpu.reg_get(self.cpu.mode(), reg::PC)) {
                return Ok(StopReason::DoneStep);
            }
        }
    }
}
