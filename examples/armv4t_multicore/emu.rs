//! ------------------------------------------------------------------------ !//
//! ------------------------------ DISCLAIMER ------------------------------ !//
//! ------------------------------------------------------------------------ !//
//!
//! This code is absolutely awful, and completely slapped together for the sake
//! of example. The watchpoint implementation is particularly awful.
//!
//! While it technically "gets the job done" and provides a simple multicore
//! system that can be debugged, it would really merit a re-write, since it's
//! not a good example of "proper Rust coding practices"

use std::collections::HashMap;

use armv4t_emu::{reg, Cpu, ExampleMem, Memory, Mode};

use crate::mem_sniffer::{AccessKind, MemSniffer};
use crate::DynResult;

const HLE_RETURN_ADDR: u32 = 0x12345678;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CpuId {
    Cpu,
    Cop,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Event {
    DoneStep,
    Halted,
    Break,
    WatchWrite(u32),
    WatchRead(u32),
}

pub enum ExecMode {
    Step,
    Continue,
}

/// incredibly barebones armv4t-based emulator
pub struct Emu {
    pub(crate) cpu: Cpu,
    pub(crate) cop: Cpu,
    pub(crate) mem: ExampleMem,

    pub(crate) exec_mode: HashMap<CpuId, ExecMode>,

    pub(crate) watchpoints: Vec<u32>,
    /// (read, write)
    pub(crate) watchpoint_kind: HashMap<u32, (bool, bool)>,
    pub(crate) breakpoints: Vec<u32>,

    // GDB seems to get gets very confused if two threads are executing the exact same code at the
    // exact same time. Maybe this is a bug with `gdbstub`?
    stall_cop_cycles: usize,
}

impl Emu {
    pub fn new(program_elf: &[u8]) -> DynResult<Emu> {
        // set up emulated system
        let mut cpu = Cpu::new();
        let mut mem = ExampleMem::new();

        // load ELF
        let elf_header = goblin::elf::Elf::parse(program_elf)?;

        // copy all in-memory sections from the ELF file into system RAM
        let sections = elf_header
            .section_headers
            .iter()
            .filter(|h| h.is_alloc() && h.sh_type != goblin::elf::section_header::SHT_NOBITS);

        for h in sections {
            eprintln!(
                "loading section {:?} into memory from [{:#010x?}..{:#010x?}]",
                elf_header.shdr_strtab.get_at(h.sh_name).unwrap(),
                h.sh_addr,
                h.sh_addr + h.sh_size,
            );

            for (i, b) in program_elf[h.file_range().unwrap()].iter().enumerate() {
                mem.w8(h.sh_addr as u32 + i as u32, *b);
            }
        }

        // setup execution state
        eprintln!("Setting PC to {:#010x?}", elf_header.entry);
        cpu.reg_set(Mode::User, reg::SP, 0x10000000);
        cpu.reg_set(Mode::User, reg::LR, HLE_RETURN_ADDR);
        cpu.reg_set(Mode::User, reg::PC, elf_header.entry as u32);
        cpu.reg_set(Mode::User, reg::CPSR, 0x10); // user mode
        let cop = cpu;

        Ok(Emu {
            cpu,
            cop,
            mem,

            exec_mode: HashMap::new(),

            watchpoints: Vec::new(),
            watchpoint_kind: HashMap::new(),
            breakpoints: Vec::new(),

            stall_cop_cycles: 24,
        })
    }

    pub fn step_core(&mut self, id: CpuId) -> Option<Event> {
        let cpu = match id {
            CpuId::Cop if self.stall_cop_cycles != 0 => {
                self.stall_cop_cycles -= 1;
                return None;
            }
            CpuId::Cop => &mut self.cop,
            CpuId::Cpu => &mut self.cpu,
        };

        // set up magic memory location
        self.mem.w8(
            0xffff_4200,
            match id {
                CpuId::Cpu => 0xaa,
                CpuId::Cop => 0x55,
            },
        );

        let mut hit_watchpoint = None;
        let mut sniffer = MemSniffer::new(&mut self.mem, &self.watchpoints, |access| {
            hit_watchpoint = Some(access)
        });

        cpu.step(&mut sniffer);
        let pc = cpu.reg_get(Mode::User, reg::PC);

        if pc == HLE_RETURN_ADDR {
            match id {
                CpuId::Cpu => return Some(Event::Halted),
                CpuId::Cop => return Some(Event::Halted),
            }
        }

        if let Some(access) = hit_watchpoint {
            // NOTE: this isn't a particularly elegant way to do watchpoints! This works
            // fine for some example code, but don't use this as inspiration in your own
            // emulator!
            match access.kind {
                AccessKind::Read => {
                    if *self
                        .watchpoint_kind
                        .get(&access.addr)
                        .map(|(r, _w)| r)
                        .unwrap_or(&false)
                    {
                        let fixup = if cpu.thumb_mode() { 2 } else { 4 };
                        cpu.reg_set(Mode::User, reg::PC, pc - fixup);
                        return Some(Event::WatchRead(access.addr));
                    }
                }
                AccessKind::Write => {
                    if *self
                        .watchpoint_kind
                        .get(&access.addr)
                        .map(|(_r, w)| w)
                        .unwrap_or(&false)
                    {
                        let fixup = if cpu.thumb_mode() { 2 } else { 4 };
                        cpu.reg_set(Mode::User, reg::PC, pc - fixup);
                        return Some(Event::WatchWrite(access.addr));
                    }
                }
            }
        }

        if self.breakpoints.contains(&pc) {
            return Some(Event::Break);
        }

        None
    }

    pub fn step(&mut self) -> Option<(Event, CpuId)> {
        let mut evt = None;

        for id in [CpuId::Cpu, CpuId::Cop].iter().copied() {
            if let Some(event) = self.step_core(id) {
                if evt.is_none() {
                    evt = Some((event, id));
                }
            }
        }

        evt
    }

    pub fn run(&mut self, mut poll_incoming_data: impl FnMut() -> bool) -> RunEvent {
        // the underlying armv4t_multicore emulator runs both cores in lock step, so
        // when GDB requests a specific core to single-step, all we need to do is jot
        // down that we want to single-step the system, as there is no way to
        // single-step a single core while the other runs.
        //
        // In more complex emulators / implementations, this simplification is _not_
        // valid, and you should track which specific TID the GDB client requested to be
        // single-stepped, and run them appropriately.

        let should_single_step = matches!(
            self.exec_mode
                .get(&CpuId::Cpu)
                .or_else(|| self.exec_mode.get(&CpuId::Cop)),
            Some(&ExecMode::Step)
        );

        match should_single_step {
            true => match self.step() {
                Some((event, id)) => RunEvent::Event(event, id),
                None => RunEvent::Event(Event::DoneStep, CpuId::Cpu),
            },
            false => {
                let mut cycles = 0;
                loop {
                    if cycles % 1024 == 0 {
                        // poll for incoming data
                        if poll_incoming_data() {
                            break RunEvent::IncomingData;
                        }
                    }
                    cycles += 1;

                    if let Some((event, id)) = self.step() {
                        break RunEvent::Event(event, id);
                    };
                }
            }
        }
    }
}

pub enum RunEvent {
    Event(Event, CpuId),
    IncomingData,
}
