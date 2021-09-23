use armv4t_emu::{reg, Cpu, ExampleMem, Memory, Mode};

use crate::mem_sniffer::{AccessKind, MemSniffer};
use crate::DynResult;

const HLE_RETURN_ADDR: u32 = 0x12345678;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Event {
    Halted,
    Break,
    WatchWrite(u32),
    WatchRead(u32),
}

/// incredibly barebones armv4t-based emulator
pub struct Emu {
    start_addr: u32,

    // example custom register. only read/written to from the GDB client
    pub(crate) custom_reg: u32,

    pub(crate) cpu: Cpu,
    pub(crate) mem: ExampleMem,

    pub(crate) watchpoints: Vec<u32>,
    pub(crate) breakpoints: Vec<u32>,
    pub(crate) files: Vec<Option<std::fs::File>>,
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

        Ok(Emu {
            start_addr: elf_header.entry as u32,

            custom_reg: 0x12345678,

            cpu,
            mem,

            watchpoints: Vec::new(),
            breakpoints: Vec::new(),
            files: Vec::new(),
        })
    }

    pub(crate) fn reset(&mut self) {
        self.cpu.reg_set(Mode::User, reg::SP, 0x10000000);
        self.cpu.reg_set(Mode::User, reg::LR, HLE_RETURN_ADDR);
        self.cpu.reg_set(Mode::User, reg::PC, self.start_addr);
        self.cpu.reg_set(Mode::User, reg::CPSR, 0x10);
    }

    pub fn step(&mut self) -> Option<Event> {
        let mut hit_watchpoint = None;

        let mut sniffer = MemSniffer::new(&mut self.mem, &self.watchpoints, |access| {
            hit_watchpoint = Some(access)
        });

        self.cpu.step(&mut sniffer);
        let pc = self.cpu.reg_get(Mode::User, reg::PC);

        if let Some(access) = hit_watchpoint {
            let fixup = if self.cpu.thumb_mode() { 2 } else { 4 };
            self.cpu.reg_set(Mode::User, reg::PC, pc - fixup);

            return Some(match access.kind {
                AccessKind::Read => Event::WatchRead(access.addr),
                AccessKind::Write => Event::WatchWrite(access.addr),
            });
        }

        if self.breakpoints.contains(&pc) {
            return Some(Event::Break);
        }

        if pc == HLE_RETURN_ADDR {
            return Some(Event::Halted);
        }

        None
    }
}
