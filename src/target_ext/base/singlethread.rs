//! Base debugging operations for single threaded targets.

use crate::arch::Arch;
use crate::target::{Target, TargetResult};
use crate::target_ext::breakpoints::WatchKind;

// Convenient re-export
pub use super::ResumeAction;

/// Base debugging operations for single threaded targets.
#[allow(clippy::type_complexity)]
pub trait SingleThreadOps: Target {
    /// Resume execution on the target.
    ///
    /// `action` specifies how the target should be resumed (i.e:
    /// single-step vs. full continue).
    ///
    /// The `check_gdb_interrupt` callback can be invoked to check if GDB sent
    /// an Interrupt packet (i.e: the user pressed Ctrl-C). It's recommended to
    /// invoke this callback every-so-often while the system is running (e.g:
    /// every X cycles/milliseconds). Periodically checking for incoming
    /// interrupt packets is _not_ required, but it is _recommended_.
    ///
    /// # Implementation requirements
    ///
    /// These requirements cannot be satisfied by `gdbstub` internally, and must
    /// be handled on a per-target basis.
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
    fn resume(
        &mut self,
        action: ResumeAction,
        check_gdb_interrupt: &mut dyn FnMut() -> bool,
    ) -> Result<StopReason<<Self::Arch as Arch>::Usize>, Self::Error>;

    /// Read the target's registers.
    fn read_registers(
        &mut self,
        regs: &mut <Self::Arch as Arch>::Registers,
    ) -> TargetResult<(), Self>;

    /// Write the target's registers.
    fn write_registers(&mut self, regs: &<Self::Arch as Arch>::Registers)
        -> TargetResult<(), Self>;

    /// Read to a single register on the target.
    ///
    /// Implementations should write the value of the register using target's
    /// native byte order in the buffer `dst`.
    ///
    /// If the requested register could not be accessed, an appropriate
    /// non-fatal error should be returned.
    ///
    /// _Note:_ This method includes a stubbed default implementation which
    /// simply returns `Ok(())`. This is due to the fact that several built-in
    /// `arch` implementations still use the generic, albeit highly un-ergonomic
    /// [`RawRegId`](../../../arch/struct.RawRegId.html) type. See the docs
    /// for `RawRegId` for more info.
    fn read_register(
        &mut self,
        reg_id: <Self::Arch as Arch>::RegId,
        dst: &mut [u8],
    ) -> TargetResult<(), Self> {
        let _ = (reg_id, dst);
        Ok(())
    }

    /// Write from a single register on the target.
    ///
    /// The `val` buffer contains the new value of the register in the target's
    /// native byte order. It is guaranteed to be the exact length as the target
    /// register.
    ///
    /// If the requested register could not be accessed, an appropriate
    /// non-fatal error should be returned.
    ///
    /// _Note:_ This method includes a stubbed default implementation which
    /// simply returns `Ok(())`. This is due to the fact that several built-in
    /// `arch` implementations still use the generic, albeit highly un-ergonomic
    /// [`RawRegId`](../../../arch/struct.RawRegId.html) type. See the docs
    /// for `RawRegId` for more info.
    fn write_register(
        &mut self,
        reg_id: <Self::Arch as Arch>::RegId,
        val: &[u8],
    ) -> TargetResult<(), Self> {
        let _ = (reg_id, val);
        Ok(())
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
}

/// Describes why the target stopped.
// NOTE: This is a simplified version of `multithread::ThreadStopReason` that omits any references
// to Tid or threads. Internally, it is converted into multithread::ThreadStopReason.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum StopReason<U> {
    /// Completed the single-step request.
    DoneStep,
    /// `check_gdb_interrupt` returned `true`
    GdbInterrupt,
    /// Halted
    Halted,
    /// Hit a software breakpoint (e.g. due to a trap instruction).
    ///
    /// NOTE: This does not necessarily have to be a breakpoint configured by
    /// the client/user of the current GDB session.
    SwBreak,
    /// Hit a hardware breakpoint.
    HwBreak,
    /// Hit a watchpoint.
    Watch {
        /// Kind of watchpoint that was hit
        kind: WatchKind,
        /// Address of watched memory
        addr: U,
    },
    /// The program received a signal
    Signal(u8),
}
