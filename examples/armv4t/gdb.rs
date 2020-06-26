use armv4t_emu::{reg, Memory};
use gdbstub::{arch, BreakOp, ResumeAction, StopReason, Target, Tid, WatchKind};

use crate::emu::{Emu, Event};

impl Target for Emu {
    type Arch = arch::arm::Armv4t;
    type Error = &'static str;

    fn resume(
        &mut self,
        mut actions: impl Iterator<Item = (Tid, ResumeAction)>,
    ) -> Result<StopReason<u32>, Self::Error> {
        // only one thread, only one action
        let (_, action) = actions.next().unwrap();

        let event = match action {
            ResumeAction::Step => self.step(),
            ResumeAction::Continue => loop {
                if let Some(event) = self.step() {
                    break Some(event);
                };
            },
        };

        let event = match event {
            Some(e) => e,
            None => return Ok(StopReason::Running),
        };

        Ok(match event {
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
        })
    }

    // order specified in binutils-gdb/blob/master/gdb/features/arm/arm-core.xml
    fn read_registers(
        &mut self,
        regs: &mut arch::arm::reg::ArmCoreRegs,
    ) -> Result<(), &'static str> {
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

    fn write_registers(&mut self, regs: &arch::arm::reg::ArmCoreRegs) -> Result<(), &'static str> {
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

    fn read_addrs(
        &mut self,
        addr: std::ops::Range<u32>,
        mut push_byte: impl FnMut(u8),
    ) -> Result<(), &'static str> {
        for addr in addr {
            push_byte(self.mem.r8(addr))
        }
        Ok(())
    }

    fn write_addrs(
        &mut self,
        mut get_addr_val: impl FnMut() -> Option<(u32, u8)>,
    ) -> Result<(), &'static str> {
        while let Some((addr, val)) = get_addr_val() {
            self.mem.w8(addr, val)
        }
        Ok(())
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
    ) -> Option<Result<bool, &'static str>> {
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
                    None => return Some(Ok(false)),
                    Some(pos) => pos,
                };

                match kind {
                    WatchKind::Write => self.watchpoints.remove(pos),
                    WatchKind::Read => self.watchpoints.remove(pos),
                    WatchKind::ReadWrite => self.watchpoints.remove(pos),
                };
            }
        }

        Some(Ok(true))
    }
}
