use armv4t_emu::{reg, Memory};
use gdbstub::{HwBreakOp, Target, TargetState, WatchKind};

use crate::emu::{Emu, Event};

impl Target for Emu {
    type Usize = u32;
    type Error = &'static str;

    fn target_description_xml() -> Option<&'static str> {
        Some(r#"<target version="1.0"><architecture>armv4t</architecture></target>"#)
    }

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
    fn read_registers(&mut self, mut push_reg: impl FnMut(&[u8])) -> Result<(), &'static str> {
        let mode = self.cpu.mode();
        for i in 0..13 {
            push_reg(&self.cpu.reg_get(mode, i).to_le_bytes());
        }
        push_reg(&self.cpu.reg_get(mode, reg::SP).to_le_bytes()); // 13
        push_reg(&self.cpu.reg_get(mode, reg::LR).to_le_bytes()); // 14
        push_reg(&self.cpu.reg_get(mode, reg::PC).to_le_bytes()); // 15

        // Floating point registers, unused
        for _ in 0..25 {
            push_reg(&[0, 0, 0, 0]);
        }

        push_reg(&self.cpu.reg_get(mode, reg::CPSR).to_le_bytes());

        Ok(())
    }

    fn write_registers(
        &mut self,
        mut pop_reg: impl FnMut() -> Option<u8>,
    ) -> Result<(), &'static str> {
        const ERR: &str = "malformed write register packet";

        let mut next = {
            move || -> Option<u32> {
                Some(
                    (pop_reg()? as u32)
                        | (pop_reg()? as u32) << 8
                        | (pop_reg()? as u32) << 16
                        | (pop_reg()? as u32) << 24,
                )
            }
        };
        let mode = self.cpu.mode();
        for i in 0..13 {
            self.cpu.reg_set(mode, i, next().ok_or(ERR)?);
        }
        self.cpu.reg_set(mode, reg::SP, next().ok_or(ERR)?);
        self.cpu.reg_set(mode, reg::LR, next().ok_or(ERR)?);
        self.cpu.reg_set(mode, reg::PC, next().ok_or(ERR)?);
        // Floating point registers, unused
        for _ in 0..25 {
            next().ok_or(ERR)?;
        }

        self.cpu.reg_set(mode, reg::CPSR, next().ok_or(ERR)?);

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

    fn update_hw_breakpoint(
        &mut self,
        addr: u32,
        op: HwBreakOp,
    ) -> Option<Result<bool, &'static str>> {
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
                    None => return Some(Ok(false)),
                    Some(pos) => pos,
                };
                self.breakpoints.remove(pos);
            }
            HwBreakOp::RemoveWatch(kind) => {
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
