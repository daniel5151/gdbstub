use crate::gdb::tracepoints::TraceFrame;
use crate::mem_sniffer::AccessKind;
use crate::mem_sniffer::MemSniffer;
use crate::DynResult;
use armv4t_emu::reg;
use armv4t_emu::Cpu;
use armv4t_emu::ExampleMem;
use armv4t_emu::Memory;
use armv4t_emu::Mode;
use gdbstub::common::Pid;
use gdbstub::target::ext::tracepoints::NewTracepoint;
use gdbstub::target::ext::tracepoints::SourceTracepoint;
use gdbstub::target::ext::tracepoints::Tracepoint;
use gdbstub::target::ext::tracepoints::TracepointAction;
use gdbstub::target::ext::tracepoints::TracepointEnumerateState;
use std::collections::HashMap;

const HLE_RETURN_ADDR: u32 = 0x12345678;

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
    RangeStep(u32, u32),
}

/// incredibly barebones armv4t-based emulator
pub struct Emu {
    start_addr: u32,

    // example custom register. only read/written to from the GDB client
    pub(crate) custom_reg: u32,

    pub(crate) exec_mode: ExecMode,

    pub(crate) cpu: Cpu,
    pub(crate) mem: ExampleMem,

    pub(crate) watchpoints: Vec<u32>,
    pub(crate) breakpoints: Vec<u32>,
    pub(crate) files: Vec<Option<std::fs::File>>,

    pub(crate) tracepoints: HashMap<
        Tracepoint,
        (
            NewTracepoint<u32>,
            Vec<SourceTracepoint<'static, u32>>,
            Vec<TracepointAction<'static, u32>>,
        ),
    >,
    pub(crate) traceframes: Vec<TraceFrame>,
    pub(crate) tracepoint_enumerate_state: TracepointEnumerateState<u32>,
    pub(crate) tracing: bool,
    pub(crate) selected_frame: Option<usize>,

    pub(crate) reported_pid: Pid,
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

            exec_mode: ExecMode::Continue,

            cpu,
            mem,

            watchpoints: Vec::new(),
            breakpoints: Vec::new(),
            files: Vec::new(),

            tracepoints: HashMap::new(),
            traceframes: Vec::new(),
            tracepoint_enumerate_state: Default::default(),
            tracing: false,
            selected_frame: None,

            reported_pid: Pid::new(1).unwrap(),
        })
    }

    pub(crate) fn reset(&mut self) {
        self.cpu.reg_set(Mode::User, reg::SP, 0x10000000);
        self.cpu.reg_set(Mode::User, reg::LR, HLE_RETURN_ADDR);
        self.cpu.reg_set(Mode::User, reg::PC, self.start_addr);
        self.cpu.reg_set(Mode::User, reg::CPSR, 0x10);
    }

    /// single-step the interpreter
    pub fn step(&mut self) -> Option<Event> {
        if self.tracing {
            let pc = self.cpu.reg_get(self.cpu.mode(), reg::PC);
            let frames: Vec<_> = self
                .tracepoints
                .iter()
                .filter(|(_tracepoint, (ctp, source, actions))| ctp.enabled && ctp.addr == pc)
                .map(|(tracepoint, _definition)| {
                    // our `tracepoint_define` restricts our loaded tracepoints to only contain
                    // register collect actions. instead of only collecting the registers requested
                    // in the register mask and recording a minimal trace frame, we just collect
                    // all of them by cloning the cpu itself.
                    TraceFrame {
                        number: *tracepoint,
                        addr: pc,
                        snapshot: self.cpu.clone(),
                    }
                })
                .collect();
            self.traceframes.extend(frames);
        }

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

    /// run the emulator in accordance with the currently set `ExecutionMode`.
    ///
    /// since the emulator runs in the same thread as the GDB loop, the emulator
    /// will use the provided callback to poll the connection for incoming data
    /// every 1024 steps.
    pub fn run(&mut self, mut poll_incoming_data: impl FnMut() -> bool) -> RunEvent {
        match self.exec_mode {
            ExecMode::Step => RunEvent::Event(self.step().unwrap_or(Event::DoneStep)),
            ExecMode::Continue => {
                let mut cycles = 0;
                loop {
                    if cycles % 1024 == 0 {
                        // poll for incoming data
                        if poll_incoming_data() {
                            break RunEvent::IncomingData;
                        }
                    }
                    cycles += 1;

                    if let Some(event) = self.step() {
                        break RunEvent::Event(event);
                    };
                }
            }
            // just continue, but with an extra PC check
            ExecMode::RangeStep(start, end) => {
                let mut cycles = 0;
                loop {
                    if cycles % 1024 == 0 {
                        // poll for incoming data
                        if poll_incoming_data() {
                            break RunEvent::IncomingData;
                        }
                    }
                    cycles += 1;

                    if let Some(event) = self.step() {
                        break RunEvent::Event(event);
                    };

                    if !(start..end).contains(&self.cpu.reg_get(self.cpu.mode(), reg::PC)) {
                        break RunEvent::Event(Event::DoneStep);
                    }
                }
            }
        }
    }
}

pub enum RunEvent {
    IncomingData,
    Event(Event),
}
