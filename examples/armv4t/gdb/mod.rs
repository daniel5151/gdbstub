use crate::emu::Emu;
use crate::emu::ExecMode;
use armv4t_emu::reg;
use armv4t_emu::Memory;
use core::convert::TryInto;
use gdbstub::common::Signal;
use gdbstub::target;
use gdbstub::target::ext::base::singlethread::SingleThreadBase;
use gdbstub::target::ext::base::singlethread::SingleThreadResume;
use gdbstub::target::Target;
use gdbstub::target::TargetError;
use gdbstub::target::TargetResult;
use gdbstub_arch::arm::reg::id::ArmCoreRegId;

// Additional GDB extensions

mod auxv;
mod breakpoints;
mod catch_syscalls;
mod exec_file;
mod extended_mode;
mod host_io;
mod libraries;
mod lldb_register_info_override;
mod memory_map;
mod monitor_cmd;
mod section_offsets;
mod target_description_xml_override;
pub(crate) mod tracepoints;

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

/// Copy all bytes of `data` to `buf`.
/// Return the size of data copied.
pub fn copy_to_buf(data: &[u8], buf: &mut [u8]) -> usize {
    let len = buf.len().min(data.len());
    buf[..len].copy_from_slice(&data[..len]);
    len
}

/// Copy a range of `data` (start at `offset` with a size of `length`) to `buf`.
/// Return the size of data copied. Returns 0 if `offset >= buf.len()`.
///
/// Mainly used by qXfer:_object_:read commands.
pub fn copy_range_to_buf(data: &[u8], offset: u64, length: usize, buf: &mut [u8]) -> usize {
    let offset = offset as usize;
    if offset > data.len() {
        return 0;
    }

    let start = offset;
    let end = (offset + length).min(data.len());
    copy_to_buf(&data[start..end], buf)
}

impl Target for Emu {
    // As an example, I've defined a custom architecture based off
    // `gdbstub_arch::arm::Armv4t`. The implementation is in the `custom_arch`
    // module at the bottom of this file.
    //
    // unless you're working with a particularly funky architecture that uses custom
    // registers, you should probably stick to using the simple `target.xml`
    // implementations from the `gdbstub_arch` repo (i.e: `target.xml` files that
    // only specify the <architecture> and <feature>s of the arch, instead of
    // listing out all the registers out manually).
    type Arch = custom_arch::Armv4tCustom;
    type Error = &'static str;

    // --------------- IMPORTANT NOTE ---------------
    // Always remember to annotate IDET enable methods with `inline(always)`!
    // Without this annotation, LLVM might fail to dead-code-eliminate nested IDET
    // implementations, resulting in unnecessary binary bloat.

    #[inline(always)]
    fn base_ops(&mut self) -> target::ext::base::BaseOps<'_, Self::Arch, Self::Error> {
        target::ext::base::BaseOps::SingleThread(self)
    }

    #[inline(always)]
    fn support_breakpoints(
        &mut self,
    ) -> Option<target::ext::breakpoints::BreakpointsOps<'_, Self>> {
        Some(self)
    }

    #[inline(always)]
    fn support_extended_mode(
        &mut self,
    ) -> Option<target::ext::extended_mode::ExtendedModeOps<'_, Self>> {
        Some(self)
    }

    #[inline(always)]
    fn support_monitor_cmd(&mut self) -> Option<target::ext::monitor_cmd::MonitorCmdOps<'_, Self>> {
        Some(self)
    }

    #[inline(always)]
    fn support_section_offsets(
        &mut self,
    ) -> Option<target::ext::section_offsets::SectionOffsetsOps<'_, Self>> {
        Some(self)
    }

    #[inline(always)]
    fn support_target_description_xml_override(
        &mut self,
    ) -> Option<
        target::ext::target_description_xml_override::TargetDescriptionXmlOverrideOps<'_, Self>,
    > {
        Some(self)
    }

    #[inline(always)]
    fn support_lldb_register_info_override(
        &mut self,
    ) -> Option<target::ext::lldb_register_info_override::LldbRegisterInfoOverrideOps<'_, Self>>
    {
        Some(self)
    }

    #[inline(always)]
    fn support_memory_map(&mut self) -> Option<target::ext::memory_map::MemoryMapOps<'_, Self>> {
        Some(self)
    }

    #[inline(always)]
    fn support_catch_syscalls(
        &mut self,
    ) -> Option<target::ext::catch_syscalls::CatchSyscallsOps<'_, Self>> {
        Some(self)
    }

    #[inline(always)]
    fn support_host_io(&mut self) -> Option<target::ext::host_io::HostIoOps<'_, Self>> {
        Some(self)
    }

    #[inline(always)]
    fn support_exec_file(&mut self) -> Option<target::ext::exec_file::ExecFileOps<'_, Self>> {
        Some(self)
    }

    #[inline(always)]
    fn support_auxv(&mut self) -> Option<target::ext::auxv::AuxvOps<'_, Self>> {
        Some(self)
    }

    #[inline(always)]
    fn support_libraries_svr4(
        &mut self,
    ) -> Option<target::ext::libraries::LibrariesSvr4Ops<'_, Self>> {
        Some(self)
    }

    #[inline(always)]
    fn support_tracepoints(
        &mut self,
    ) -> Option<target::ext::tracepoints::TracepointsOps<'_, Self>> {
        Some(self)
    }
}

impl SingleThreadBase for Emu {
    fn read_registers(
        &mut self,
        regs: &mut custom_arch::ArmCoreRegsCustom,
    ) -> TargetResult<(), Self> {
        // if we selected a frame from a tracepoint, return registers from that snapshot
        let cpu = self.selected_frame.and_then(|selected| {
            self.traceframes.get(selected)
        }).map(|frame| {
            frame.snapshot
        }).unwrap_or_else(|| self.cpu);
        let mode = cpu.mode();

        for i in 0..13 {
            regs.core.r[i] = cpu.reg_get(mode, i as u8);
        }
        regs.core.sp = cpu.reg_get(mode, reg::SP);
        regs.core.lr = cpu.reg_get(mode, reg::LR);
        regs.core.pc = cpu.reg_get(mode, reg::PC);
        regs.core.cpsr = cpu.reg_get(mode, reg::CPSR);

        regs.custom = self.custom_reg;

        Ok(())
    }

    fn write_registers(&mut self, regs: &custom_arch::ArmCoreRegsCustom) -> TargetResult<(), Self> {
        if self.selected_frame.is_some() {
            // we can't modify registers in a tracepoint frame
            return Err(TargetError::NonFatal);
        }
        let mode = self.cpu.mode();

        for i in 0..13 {
            self.cpu.reg_set(mode, i, regs.core.r[i as usize]);
        }
        self.cpu.reg_set(mode, reg::SP, regs.core.sp);
        self.cpu.reg_set(mode, reg::LR, regs.core.lr);
        self.cpu.reg_set(mode, reg::PC, regs.core.pc);
        self.cpu.reg_set(mode, reg::CPSR, regs.core.cpsr);

        self.custom_reg = regs.custom;

        Ok(())
    }

    #[inline(always)]
    fn support_single_register_access(
        &mut self,
    ) -> Option<target::ext::base::single_register_access::SingleRegisterAccessOps<'_, (), Self>>
    {
        Some(self)
    }

    fn read_addrs(&mut self, start_addr: u32, data: &mut [u8]) -> TargetResult<usize, Self> {
        if self.selected_frame.is_some() {
            // we only support register collection actions for our tracepoint frames.
            // if we have a selected frame, then we don't have any memory we can
            // return from the frame snapshot.
            return Ok(0)
        }
        // this is a simple emulator, with RAM covering the entire 32 bit address space
        for (addr, val) in (start_addr..).zip(data.iter_mut()) {
            *val = self.mem.r8(addr)
        }
        Ok(data.len())
    }

    fn write_addrs(&mut self, start_addr: u32, data: &[u8]) -> TargetResult<(), Self> {
        if self.selected_frame.is_some() {
            // we can't modify memory in a tracepoint frame
            return Err(TargetError::NonFatal);
        }

        // this is a simple emulator, with RAM covering the entire 32 bit address space
        for (addr, val) in (start_addr..).zip(data.iter().copied()) {
            self.mem.w8(addr, val)
        }
        Ok(())
    }

    #[inline(always)]
    fn support_resume(
        &mut self,
    ) -> Option<target::ext::base::singlethread::SingleThreadResumeOps<'_, Self>> {
        Some(self)
    }
}

impl SingleThreadResume for Emu {
    fn resume(&mut self, signal: Option<Signal>) -> Result<(), Self::Error> {
        // Upon returning from the `resume` method, the target being debugged should be
        // configured to run according to whatever resume actions the GDB client has
        // specified (as specified by `set_resume_action`, `resume_range_step`,
        // `reverse_{step, continue}`, etc...)
        //
        // In this basic `armv4t` example, the `resume` method simply sets the exec mode
        // of the emulator's interpreter loop and returns.
        //
        // In more complex implementations, it's likely that the target being debugged
        // will be running in another thread / process, and will require some kind of
        // external "orchestration" to set it's execution mode (e.g: modifying the
        // target's process state via platform specific debugging syscalls).

        if signal.is_some() {
            return Err("no support for continuing with signal");
        }

        self.exec_mode = ExecMode::Continue;

        Ok(())
    }

    #[inline(always)]
    fn support_reverse_cont(
        &mut self,
    ) -> Option<target::ext::base::reverse_exec::ReverseContOps<'_, (), Self>> {
        Some(self)
    }

    #[inline(always)]
    fn support_reverse_step(
        &mut self,
    ) -> Option<target::ext::base::reverse_exec::ReverseStepOps<'_, (), Self>> {
        Some(self)
    }

    #[inline(always)]
    fn support_single_step(
        &mut self,
    ) -> Option<target::ext::base::singlethread::SingleThreadSingleStepOps<'_, Self>> {
        Some(self)
    }

    #[inline(always)]
    fn support_range_step(
        &mut self,
    ) -> Option<target::ext::base::singlethread::SingleThreadRangeSteppingOps<'_, Self>> {
        Some(self)
    }
}

impl target::ext::base::singlethread::SingleThreadSingleStep for Emu {
    fn step(&mut self, signal: Option<Signal>) -> Result<(), Self::Error> {
        if signal.is_some() {
            return Err("no support for stepping with signal");
        }

        self.exec_mode = ExecMode::Step;

        Ok(())
    }
}

impl target::ext::base::single_register_access::SingleRegisterAccess<()> for Emu {
    fn read_register(
        &mut self,
        _tid: (),
        reg_id: custom_arch::ArmCoreRegIdCustom,
        buf: &mut [u8],
    ) -> TargetResult<usize, Self> {
        match reg_id {
            custom_arch::ArmCoreRegIdCustom::Core(reg_id) => {
                if let Some(i) = cpu_reg_id(reg_id) {
                    let w = self.cpu.reg_get(self.cpu.mode(), i);
                    buf.copy_from_slice(&w.to_le_bytes());
                    Ok(buf.len())
                } else {
                    Err(().into())
                }
            }
            custom_arch::ArmCoreRegIdCustom::Custom => {
                buf.copy_from_slice(&self.custom_reg.to_le_bytes());
                Ok(buf.len())
            }
            custom_arch::ArmCoreRegIdCustom::Time => {
                buf.copy_from_slice(
                    &(std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_millis() as u32)
                        .to_le_bytes(),
                );
                Ok(buf.len())
            }
            custom_arch::ArmCoreRegIdCustom::Unavailable => Ok(0),
        }
    }

    fn write_register(
        &mut self,
        _tid: (),
        reg_id: custom_arch::ArmCoreRegIdCustom,
        val: &[u8],
    ) -> TargetResult<(), Self> {
        let w = u32::from_le_bytes(
            val.try_into()
                .map_err(|_| TargetError::Fatal("invalid data"))?,
        );
        match reg_id {
            custom_arch::ArmCoreRegIdCustom::Core(reg_id) => {
                if let Some(i) = cpu_reg_id(reg_id) {
                    self.cpu.reg_set(self.cpu.mode(), i, w);
                    Ok(())
                } else {
                    Err(().into())
                }
            }
            custom_arch::ArmCoreRegIdCustom::Custom => {
                self.custom_reg = w;
                Ok(())
            }
            // ignore writes
            custom_arch::ArmCoreRegIdCustom::Unavailable
            | custom_arch::ArmCoreRegIdCustom::Time => Ok(()),
        }
    }
}

impl target::ext::base::reverse_exec::ReverseCont<()> for Emu {
    fn reverse_cont(&mut self) -> Result<(), Self::Error> {
        // FIXME: actually implement reverse step
        eprintln!(
            "FIXME: Not actually reverse-continuing. Performing forwards continue instead..."
        );
        self.exec_mode = ExecMode::Continue;
        Ok(())
    }
}

impl target::ext::base::reverse_exec::ReverseStep<()> for Emu {
    fn reverse_step(&mut self, _tid: ()) -> Result<(), Self::Error> {
        // FIXME: actually implement reverse step
        eprintln!(
            "FIXME: Not actually reverse-stepping. Performing single forwards step instead..."
        );
        self.exec_mode = ExecMode::Step;
        Ok(())
    }
}

impl target::ext::base::singlethread::SingleThreadRangeStepping for Emu {
    fn resume_range_step(&mut self, start: u32, end: u32) -> Result<(), Self::Error> {
        self.exec_mode = ExecMode::RangeStep(start, end);
        Ok(())
    }
}

mod custom_arch {
    use core::num::NonZeroUsize;
    use gdbstub::arch::lldb::Encoding;
    use gdbstub::arch::lldb::Format;
    use gdbstub::arch::lldb::Generic;
    use gdbstub::arch::lldb::Register;
    use gdbstub::arch::lldb::RegisterInfo;
    use gdbstub::arch::Arch;
    use gdbstub::arch::RegId;
    use gdbstub::arch::Registers;
    use gdbstub_arch::arm::reg::id::ArmCoreRegId;
    use gdbstub_arch::arm::reg::ArmCoreRegs;
    use gdbstub_arch::arm::ArmBreakpointKind;

    /// Implements `Arch` for ARMv4T
    pub enum Armv4tCustom {}

    #[derive(Debug, Default, Clone, Eq, PartialEq)]
    pub struct ArmCoreRegsCustom {
        pub core: ArmCoreRegs,
        pub custom: u32,
    }

    impl Registers for ArmCoreRegsCustom {
        type ProgramCounter = u32;

        fn pc(&self) -> Self::ProgramCounter {
            self.core.pc
        }

        fn gdb_serialize(&self, mut write_byte: impl FnMut(Option<u8>)) {
            self.core.gdb_serialize(&mut write_byte);

            macro_rules! write_bytes {
                ($bytes:expr) => {
                    for b in $bytes {
                        write_byte(Some(*b))
                    }
                };
            }

            write_bytes!(&self.custom.to_le_bytes());
        }

        fn gdb_deserialize(&mut self, bytes: &[u8]) -> Result<(), ()> {
            // ensure bytes.chunks_exact(4) won't panic
            if bytes.len() % 4 != 0 {
                return Err(());
            }

            use core::convert::TryInto;
            let mut regs = bytes
                .chunks_exact(4)
                .map(|c| u32::from_le_bytes(c.try_into().unwrap()));

            // copied from ArmCoreRegs
            {
                for reg in self.core.r.iter_mut() {
                    *reg = regs.next().ok_or(())?
                }
                self.core.sp = regs.next().ok_or(())?;
                self.core.lr = regs.next().ok_or(())?;
                self.core.pc = regs.next().ok_or(())?;

                // Floating point registers (unused)
                for _ in 0..25 {
                    regs.next().ok_or(())?;
                }

                self.core.cpsr = regs.next().ok_or(())?;
            }

            self.custom = regs.next().ok_or(())?;

            if regs.next().is_some() {
                return Err(());
            }

            Ok(())
        }
    }

    #[derive(Debug)]
    pub enum ArmCoreRegIdCustom {
        Core(ArmCoreRegId),
        Custom,
        // not sent as part of `struct ArmCoreRegsCustom`, and only accessible via the single
        // register read/write functions
        Time,
        /// This pseudo-register is valid but never available
        Unavailable,
    }

    impl RegId for ArmCoreRegIdCustom {
        fn from_raw_id(id: usize) -> Option<(Self, Option<NonZeroUsize>)> {
            let reg = match id {
                26 => Self::Custom,
                27 => Self::Time,
                28 => Self::Unavailable,
                _ => {
                    let (reg, size) = ArmCoreRegId::from_raw_id(id)?;
                    return Some((Self::Core(reg), size));
                }
            };
            Some((reg, Some(NonZeroUsize::new(4)?)))
        }
    }

    impl Arch for Armv4tCustom {
        type Usize = u32;
        type Registers = ArmCoreRegsCustom;
        type RegId = ArmCoreRegIdCustom;
        type BreakpointKind = ArmBreakpointKind;

        // for _purely demonstrative purposes_, i'll return dummy data from this
        // function, as it will be overwritten by TargetDescriptionXmlOverride.
        //
        // See `examples/armv4t/gdb/target_description_xml_override.rs`
        //
        // in an actual implementation, you'll want to return an actual string here!
        fn target_description_xml() -> Option<&'static str> {
            Some("never gets returned")
        }

        // (LLDB extension)
        //
        // for _purely demonstrative purposes_, even though this provides a working
        // example, it will get overwritten by RegisterInfoOverride.
        //
        // See `examples/armv4t/gdb/register_info_override.rs`
        fn lldb_register_info(reg_id: usize) -> Option<RegisterInfo<'static>> {
            match ArmCoreRegIdCustom::from_raw_id(reg_id) {
                Some((_, None)) | None => Some(RegisterInfo::Done),
                Some((r, Some(size))) => {
                    let name = match r {
                        // For the purpose of demonstration, we end the qRegisterInfo packet
                        // exchange when reaching the Time register id, so that this register can
                        // only be explicitly queried via the single-register read packet.
                        ArmCoreRegIdCustom::Time => return Some(RegisterInfo::Done),
                        ArmCoreRegIdCustom::Core(ArmCoreRegId::Gpr(i)) => match i {
                            0 => "r0",
                            1 => "r1",
                            2 => "r2",
                            3 => "r3",
                            4 => "r4",
                            5 => "r5",
                            6 => "r6",
                            7 => "r7",
                            8 => "r8",
                            9 => "r9",
                            10 => "r10",
                            11 => "r11",
                            12 => "r12",
                            _ => "unknown",
                        },
                        ArmCoreRegIdCustom::Core(ArmCoreRegId::Sp) => "sp",
                        ArmCoreRegIdCustom::Core(ArmCoreRegId::Lr) => "lr",
                        ArmCoreRegIdCustom::Core(ArmCoreRegId::Pc) => "pc",
                        ArmCoreRegIdCustom::Core(ArmCoreRegId::Fpr(_i)) => "padding",
                        ArmCoreRegIdCustom::Core(ArmCoreRegId::Fps) => "padding",
                        ArmCoreRegIdCustom::Core(ArmCoreRegId::Cpsr) => "cpsr",
                        ArmCoreRegIdCustom::Custom => "custom",
                        ArmCoreRegIdCustom::Unavailable => "Unavailable",
                        _ => "unknown",
                    };
                    let encoding = match r {
                        ArmCoreRegIdCustom::Core(ArmCoreRegId::Gpr(_i)) => Encoding::Uint,
                        ArmCoreRegIdCustom::Core(ArmCoreRegId::Sp)
                        | ArmCoreRegIdCustom::Core(ArmCoreRegId::Pc)
                        | ArmCoreRegIdCustom::Core(ArmCoreRegId::Cpsr)
                        | ArmCoreRegIdCustom::Unavailable
                        | ArmCoreRegIdCustom::Custom => Encoding::Uint,
                        _ => Encoding::Vector,
                    };
                    let format = match r {
                        ArmCoreRegIdCustom::Core(ArmCoreRegId::Gpr(_i)) => Format::Hex,
                        ArmCoreRegIdCustom::Core(ArmCoreRegId::Sp)
                        | ArmCoreRegIdCustom::Core(ArmCoreRegId::Pc)
                        | ArmCoreRegIdCustom::Core(ArmCoreRegId::Cpsr)
                        | ArmCoreRegIdCustom::Unavailable
                        | ArmCoreRegIdCustom::Custom => Format::Hex,
                        _ => Format::VectorUInt8,
                    };
                    let set = match r {
                        ArmCoreRegIdCustom::Core(ArmCoreRegId::Gpr(_i)) => {
                            "General Purpose Registers"
                        }
                        ArmCoreRegIdCustom::Core(ArmCoreRegId::Sp)
                        | ArmCoreRegIdCustom::Core(ArmCoreRegId::Pc)
                        | ArmCoreRegIdCustom::Core(ArmCoreRegId::Cpsr)
                        | ArmCoreRegIdCustom::Unavailable
                        | ArmCoreRegIdCustom::Custom => "General Purpose Registers",
                        _ => "Floating Point Registers",
                    };
                    let generic = match r {
                        ArmCoreRegIdCustom::Core(ArmCoreRegId::Sp) => Some(Generic::Sp),
                        ArmCoreRegIdCustom::Core(ArmCoreRegId::Pc) => Some(Generic::Pc),
                        _ => None,
                    };
                    let reg = Register {
                        name,
                        alt_name: None,
                        bitsize: (usize::from(size)) * 8,
                        offset: reg_id * (usize::from(size)),
                        encoding,
                        format,
                        set,
                        gcc: None,
                        dwarf: Some(reg_id),
                        generic,
                        container_regs: None,
                        invalidate_regs: None,
                    };
                    Some(RegisterInfo::Register(reg))
                }
            }
        }
    }
}
