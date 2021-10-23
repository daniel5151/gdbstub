use core::convert::{TryFrom, TryInto};

use armv4t_emu::{reg, Memory};
use gdbstub::common::Signal;
use gdbstub::target;
use gdbstub::target::ext::base::singlethread::SingleThreadOps;
use gdbstub::target::{Target, TargetError, TargetResult};
use gdbstub_arch::arm::reg::id::ArmCoreRegId;

use crate::emu::{Emu, ExecMode};

// Additional GDB extensions

mod auxv;
mod breakpoints;
mod catch_syscalls;
mod exec_file;
mod extended_mode;
mod host_io;
mod memory_map;
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

/// Copy all bytes of `data` to `buf`.
/// Return the size of data copied.
pub fn copy_to_buf(data: &[u8], buf: &mut [u8]) -> usize {
    let len = data.len();
    let buf = &mut buf[..len];
    buf.copy_from_slice(data);
    len
}

/// Copy a range of `data` (start at `offset` with a size of `length`) to `buf`.
/// Return the size of data copied. Returns 0 if `offset >= buf.len()`.
///
/// Mainly used by qXfer:_object_:read commands.
pub fn copy_range_to_buf(data: &[u8], offset: u64, length: usize, buf: &mut [u8]) -> usize {
    let offset = match usize::try_from(offset) {
        Ok(v) => v,
        Err(_) => return 0,
    };
    let len = data.len();
    let data = &data[len.min(offset)..len.min(offset + length)];
    copy_to_buf(data, buf)
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
    fn base_ops(&mut self) -> target::ext::base::BaseOps<Self::Arch, Self::Error> {
        target::ext::base::BaseOps::SingleThread(self)
    }

    #[inline(always)]
    fn support_breakpoints(&mut self) -> Option<target::ext::breakpoints::BreakpointsOps<Self>> {
        Some(self)
    }

    #[inline(always)]
    fn support_extended_mode(
        &mut self,
    ) -> Option<target::ext::extended_mode::ExtendedModeOps<Self>> {
        Some(self)
    }

    #[inline(always)]
    fn support_monitor_cmd(&mut self) -> Option<target::ext::monitor_cmd::MonitorCmdOps<Self>> {
        Some(self)
    }

    #[inline(always)]
    fn support_section_offsets(
        &mut self,
    ) -> Option<target::ext::section_offsets::SectionOffsetsOps<Self>> {
        Some(self)
    }

    #[inline(always)]
    fn support_target_description_xml_override(
        &mut self,
    ) -> Option<target::ext::target_description_xml_override::TargetDescriptionXmlOverrideOps<Self>>
    {
        Some(self)
    }

    #[inline(always)]
    fn support_memory_map(&mut self) -> Option<target::ext::memory_map::MemoryMapOps<Self>> {
        Some(self)
    }

    #[inline(always)]
    fn support_catch_syscalls(
        &mut self,
    ) -> Option<target::ext::catch_syscalls::CatchSyscallsOps<Self>> {
        Some(self)
    }

    #[inline(always)]
    fn support_host_io(&mut self) -> Option<target::ext::host_io::HostIoOps<Self>> {
        Some(self)
    }

    #[inline(always)]
    fn support_exec_file(&mut self) -> Option<target::ext::exec_file::ExecFileOps<Self>> {
        Some(self)
    }

    #[inline(always)]
    fn support_auxv(&mut self) -> Option<target::ext::auxv::AuxvOps<Self>> {
        Some(self)
    }
}

impl SingleThreadOps for Emu {
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

    fn read_registers(
        &mut self,
        regs: &mut custom_arch::ArmCoreRegsCustom,
    ) -> TargetResult<(), Self> {
        let mode = self.cpu.mode();

        for i in 0..13 {
            regs.core.r[i] = self.cpu.reg_get(mode, i as u8);
        }
        regs.core.sp = self.cpu.reg_get(mode, reg::SP);
        regs.core.lr = self.cpu.reg_get(mode, reg::LR);
        regs.core.pc = self.cpu.reg_get(mode, reg::PC);
        regs.core.cpsr = self.cpu.reg_get(mode, reg::CPSR);

        regs.custom = self.custom_reg;

        Ok(())
    }

    fn write_registers(&mut self, regs: &custom_arch::ArmCoreRegsCustom) -> TargetResult<(), Self> {
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
    fn support_single_register_access(
        &mut self,
    ) -> Option<target::ext::base::SingleRegisterAccessOps<(), Self>> {
        Some(self)
    }

    #[inline(always)]
    fn support_reverse_cont(
        &mut self,
    ) -> Option<target::ext::base::singlethread::SingleThreadReverseContOps<Self>> {
        Some(self)
    }

    #[inline(always)]
    fn support_reverse_step(
        &mut self,
    ) -> Option<target::ext::base::singlethread::SingleThreadReverseStepOps<Self>> {
        Some(self)
    }

    #[inline(always)]
    fn support_single_step(
        &mut self,
    ) -> Option<target::ext::base::singlethread::SingleThreadSingleStepOps<Self>> {
        Some(self)
    }

    #[inline(always)]
    fn support_range_step(
        &mut self,
    ) -> Option<target::ext::base::singlethread::SingleThreadRangeSteppingOps<Self>> {
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

impl target::ext::base::SingleRegisterAccess<()> for Emu {
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
            custom_arch::ArmCoreRegIdCustom::Time => Ok(()),
        }
    }
}

impl target::ext::base::singlethread::SingleThreadReverseCont for Emu {
    fn reverse_cont(&mut self) -> Result<(), Self::Error> {
        // FIXME: actually implement reverse step
        eprintln!(
            "FIXME: Not actually reverse-continuing. Performing forwards continue instead..."
        );
        self.exec_mode = ExecMode::Continue;
        Ok(())
    }
}

impl target::ext::base::singlethread::SingleThreadReverseStep for Emu {
    fn reverse_step(&mut self) -> Result<(), Self::Error> {
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

    use gdbstub::arch::{Arch, RegId, Registers};

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
    }

    impl RegId for ArmCoreRegIdCustom {
        fn from_raw_id(id: usize) -> Option<(Self, Option<NonZeroUsize>)> {
            let reg = match id {
                26 => Self::Custom,
                27 => Self::Time,
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

        // armv4t supports optional single stepping.
        //
        // notably, x86 is an example of an arch that does _not_ support
        // optional single stepping.
        fn supports_optional_single_step() -> bool {
            true
        }
    }
}
