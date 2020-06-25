use armv4t_emu::{reg, Memory};
use gdbstub::{arch, HwBreakOp, Target, TargetState, WatchKind};

use crate::emu::{Emu, Event};

impl Target for Emu {
    type Arch = arch::arm::Armv4t;
    type Error = &'static str;

    fn step(&mut self) -> Result<TargetState<u32>, Self::Error> {
        let event = match self.step() {
            Some(event) => event,
            None => return Ok(TargetState::Running),
        };

        Ok(match event {
            Event::Halted => TargetState::Halted,
            Event::Break => TargetState::HwBreak,
            Event::WatchWrite(addr) => TargetState::Watch {
                kind: WatchKind::Write,
                addr,
            },
            Event::WatchRead(addr) => TargetState::Watch {
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

    fn read_pc(&mut self) -> Result<u32, &'static str> {
        Ok(self.cpu.reg_get(self.cpu.mode(), reg::PC))
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

    fn impl_update_hw_breakpoint(&self) -> bool {
        true
    }

    fn update_hw_breakpoint(&mut self, addr: u32, op: HwBreakOp) -> Result<bool, &'static str> {
        match op {
            HwBreakOp::AddBreak => self.breakpoints.push(addr),
            HwBreakOp::AddWatch(kind) => {
                match kind {
                    WatchKind::Write => self.watchpoints.push(addr),
                    WatchKind::Read => self.watchpoints.push(addr),
                    WatchKind::ReadWrite => self.watchpoints.push(addr),
                };
            }
            HwBreakOp::RemoveBreak => {
                let pos = match self.breakpoints.iter().position(|x| *x == addr) {
                    None => return Ok(false),
                    Some(pos) => pos,
                };
                self.breakpoints.remove(pos);
            }
            HwBreakOp::RemoveWatch(kind) => {
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
}
