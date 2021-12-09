//! Base debugging operations for single threaded targets.

use crate::arch::Arch;
use crate::common::Signal;
use crate::target::{Target, TargetResult};

/// Base required debugging operations for single threaded targets.
pub trait SingleThreadBase: Target {
    /// Read the target's registers.
    fn read_registers(
        &mut self,
        regs: &mut <Self::Arch as Arch>::Registers,
    ) -> TargetResult<(), Self>;

    /// Write the target's registers.
    fn write_registers(&mut self, regs: &<Self::Arch as Arch>::Registers)
        -> TargetResult<(), Self>;

    /// Support for single-register access.
    /// See [`SingleRegisterAccess`] for more details.
    ///
    /// While this is an optional feature, it is **highly recommended** to
    /// implement it when possible, as it can significantly improve performance
    /// on certain architectures.
    ///
    /// [`SingleRegisterAccess`]:
    /// super::single_register_access::SingleRegisterAccess
    #[inline(always)]
    fn support_single_register_access(
        &mut self,
    ) -> Option<super::single_register_access::SingleRegisterAccessOps<'_, (), Self>> {
        None
    }

    /// Read bytes from the specified address range.
    ///
    /// If the requested address range could not be accessed (e.g: due to
    /// MMU protection, unhanded page fault, etc...), an appropriate
    /// non-fatal error should be returned.
    fn read_addrs(
        &mut self,
        start_addr: <Self::Arch as Arch>::Usize,
        data: &mut [u8],
    ) -> TargetResult<(), Self>;

    /// Write bytes to the specified address range.
    ///
    /// If the requested address range could not be accessed (e.g: due to
    /// MMU protection, unhanded page fault, etc...), an appropriate
    /// non-fatal error should be returned.
    fn write_addrs(
        &mut self,
        start_addr: <Self::Arch as Arch>::Usize,
        data: &[u8],
    ) -> TargetResult<(), Self>;

    /// Support for resuming the target (e.g: via `continue` or `step`)
    #[inline(always)]
    fn support_resume(&mut self) -> Option<SingleThreadResumeOps<'_, Self>> {
        None
    }
}

/// Target extension - support for resuming single threaded targets.
pub trait SingleThreadResume: Target {
    /// Resume execution on the target.
    ///
    /// The GDB client may also include a `signal` which should be passed to the
    /// target.
    ///
    /// # Additional Considerations
    ///
    /// ### Adjusting PC after a breakpoint is hit
    ///
    /// The [GDB remote serial protocol documentation](https://sourceware.org/gdb/current/onlinedocs/gdb/Stop-Reply-Packets.html#swbreak-stop-reason)
    /// notes the following:
    ///
    /// > On some architectures, such as x86, at the architecture level, when a
    /// > breakpoint instruction executes the program counter points at the
    /// > breakpoint address plus an offset. On such targets, the stub is
    /// > responsible for adjusting the PC to point back at the breakpoint
    /// > address.
    ///
    /// Omitting PC adjustment may result in unexpected execution flow and/or
    /// breakpoints not appearing to work correctly.
    fn resume(&mut self, signal: Option<Signal>) -> Result<(), Self::Error>;

    /// Support for optimized [single stepping].
    ///
    /// [single stepping]: https://sourceware.org/gdb/current/onlinedocs/gdb/Continuing-and-Stepping.html#index-stepi
    #[inline(always)]
    fn support_single_step(&mut self) -> Option<SingleThreadSingleStepOps<'_, Self>> {
        None
    }

    /// Support for optimized [range stepping].
    ///
    /// [range stepping]: https://sourceware.org/gdb/current/onlinedocs/gdb/Continuing-and-Stepping.html#range-stepping
    #[inline(always)]
    fn support_range_step(&mut self) -> Option<SingleThreadRangeSteppingOps<'_, Self>> {
        None
    }

    /// Support for [reverse stepping] a target.
    ///
    /// [reverse stepping]: https://sourceware.org/gdb/current/onlinedocs/gdb/Reverse-Execution.html
    #[inline(always)]
    fn support_reverse_step(
        &mut self,
    ) -> Option<super::reverse_exec::ReverseStepOps<'_, (), Self>> {
        None
    }

    /// Support for [reverse continuing] a target.
    ///
    /// [reverse continuing]: https://sourceware.org/gdb/current/onlinedocs/gdb/Reverse-Execution.html
    #[inline(always)]
    fn support_reverse_cont(
        &mut self,
    ) -> Option<super::reverse_exec::ReverseContOps<'_, (), Self>> {
        None
    }
}

define_ext!(SingleThreadResumeOps, SingleThreadResume);

/// Target Extension - Optimized [single stepping] for single threaded targets.
/// See [`SingleThreadResume::support_single_step`].
pub trait SingleThreadSingleStep: Target + SingleThreadResume {
    /// [Single step] the target.
    ///
    /// Single stepping will step the target a single "step" - typically a
    /// single instruction.
    /// The GDB client may also include a `signal` which should be passed to the
    /// target.
    ///
    /// [Single step]: https://sourceware.org/gdb/current/onlinedocs/gdb/Continuing-and-Stepping.html#index-stepi
    fn step(&mut self, signal: Option<Signal>) -> Result<(), Self::Error>;
}

define_ext!(SingleThreadSingleStepOps, SingleThreadSingleStep);

/// Target Extension - Optimized range stepping for single threaded targets.
/// See [`SingleThreadResume::support_range_step`].
pub trait SingleThreadRangeStepping: Target + SingleThreadResume {
    /// [Range step] the target.
    ///
    /// Range Stepping will step the target once, and keep stepping the target
    /// as long as execution remains between the specified start (inclusive)
    /// and end (exclusive) addresses, or another stop condition is met
    /// (e.g: a breakpoint it hit).
    ///
    /// If the range is empty (`start` == `end`), then the action becomes
    /// equivalent to the ‘s’ action. In other words, single-step once, and
    /// report the stop (even if the stepped instruction jumps to start).
    ///
    /// _Note:_ A stop reply may be sent at any point even if the PC is still
    /// within the stepping range; for example, it is valid to implement range
    /// stepping in a degenerate way as a single instruction step operation.
    ///
    /// [Range step]: https://sourceware.org/gdb/current/onlinedocs/gdb/Continuing-and-Stepping.html#range-stepping
    fn resume_range_step(
        &mut self,
        start: <Self::Arch as Arch>::Usize,
        end: <Self::Arch as Arch>::Usize,
    ) -> Result<(), Self::Error>;
}

define_ext!(SingleThreadRangeSteppingOps, SingleThreadRangeStepping);
