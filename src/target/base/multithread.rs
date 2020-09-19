//! Base debugging operations for multi threaded targets.

use crate::arch::Arch;
use crate::target::ext::breakpoint::WatchKind;
use crate::target::Target;
use crate::Tid;

// Convenient re-exports
pub use super::ResumeAction;

/// Selects a thread corresponding to a ResumeAction.
// NOTE: this is a subset of the internal `IdKind` type, albeit without an `Any` variant. Selecting
// `Any` thread is something that's handled by `gdbstub` internally, and shouldn't be exposed to the
// end user.
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum TidSelector {
    /// Thread with a specific ID.
    WithID(Tid),
    /// All (other) threads.
    All,
}

/// Base debugging operations for multi threaded targets.
#[allow(clippy::type_complexity)]
pub trait MultiThreadOps: Target {
    /// Resume execution on the target.
    ///
    /// `actions` is an iterator over `(TidSelector, ResumeAction)` pairs which
    /// specify how various threads should be resumed (i.e: single-step vs.
    /// resume). It is _guaranteed_ to contain at least one action. It is not
    /// guaranteed to be exhaustive over all live threads, and any threads
    /// without a corresponding `TidSelector` should be left in the same state
    /// (if possible).
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
    ///
    /// # Additional Considerations
    ///
    /// ### "Non-stop" mode
    ///
    /// At the moment, `gdbstub` only supports GDB's
    /// ["All-Stop" mode](https://sourceware.org/gdb/current/onlinedocs/gdb/All_002dStop-Mode.html),
    /// whereby _all_ threads should be stopped when returning from `resume`
    /// (not just the thread associated with the `ThreadStopReason`).
    ///
    /// ### Bare-Metal Targets
    ///
    /// On bare-metal targets (such as microcontrollers or emulators), it's
    /// common to treat individual _CPU cores_ as a separate "threads". e.g:
    /// in a dual-core system, [CPU0, CPU1] might be mapped to [TID1, TID2]
    /// (note that TIDs cannot be zero).
    ///
    /// In this case, the `Tid` argument of `read/write_addrs` becomes quite
    /// relevant, as different cores may have different memory maps.
    fn resume(
        &mut self,
        actions: Actions<'_>,
        check_gdb_interrupt: &mut dyn FnMut() -> bool,
    ) -> Result<ThreadStopReason<<Self::Arch as Arch>::Usize>, Self::Error>;

    /// Read the target's registers.
    fn read_registers(
        &mut self,
        regs: &mut <Self::Arch as Arch>::Registers,
        tid: Tid,
    ) -> Result<(), Self::Error>;

    /// Write the target's registers.
    fn write_registers(
        &mut self,
        regs: &<Self::Arch as Arch>::Registers,
        tid: Tid,
    ) -> Result<(), Self::Error>;

    /// Read to a single register on the target.
    ///
    /// Implementations should write the value of the register using target's
    /// native byte order in the buffer `dst`.
    ///
    /// If the requested register could not be accessed, return `Ok(false)` to
    /// signal that the requested register could not be read from. Otherwise,
    /// return `Ok(true)`.
    ///
    /// As a reminder, `Err(Self::Error)` should only be returned if a register
    /// read results in a **fatal** target error.
    ///
    /// _Note:_ This method includes a stubbed default implementation which
    /// simply returns `Ok(false)`. This is due to the fact that several
    /// built-in `arch` implementations still use the generic, albeit highly
    /// un-ergonomic [`RawRegId`](../../../arch/struct.RawRegId.html)
    /// type. See the docs for `RawRegId` for more info.
    fn read_register(
        &mut self,
        reg_id: <Self::Arch as Arch>::RegId,
        dst: &mut [u8],
        tid: Tid,
    ) -> Result<bool, Self::Error> {
        let _ = (reg_id, dst, tid);
        Ok(false)
    }

    /// Write from a single register on the target.
    ///
    /// The `val` buffer contains the new value of the register in the target's
    /// native byte order. It is guaranteed to be the exact length as the target
    /// register.
    ///
    /// If the requested register could not be accessed, return `Ok(false)` to
    /// signal that the requested register could not be written to. Otherwise,
    /// return `Ok(true)`.
    ///
    /// As a reminder, `Err(Self::Error)` should only be returned if a register
    /// read results in a **fatal** target error.
    ///
    /// _Note:_ This method includes a stubbed default implementation which
    /// simply returns `Ok(false)`. This is due to the fact that several
    /// built-in `arch` implementations still use the generic, albeit highly
    /// un-ergonomic [`RawRegId`](../../../arch/struct.RawRegId.html)
    /// type. See the docs for `RawRegId` for more info.
    fn write_register(
        &mut self,
        reg_id: <Self::Arch as Arch>::RegId,
        val: &[u8],
        tid: Tid,
    ) -> Result<bool, Self::Error> {
        let _ = (reg_id, val, tid);
        Ok(false)
    }

    /// Read bytes from the specified address range.
    ///
    /// If the requested address range could not be accessed (e.g: due to
    /// MMU protection, unhanded page fault, etc...), return `Ok(false)` to
    /// signal that the requested memory could not be read.
    ///
    /// As a reminder, `Err(Self::Error)` should only be returned if a memory
    /// read results in a **fatal** target error.
    fn read_addrs(
        &mut self,
        start_addr: <Self::Arch as Arch>::Usize,
        data: &mut [u8],
        tid: Tid,
    ) -> Result<bool, Self::Error>;

    /// Write bytes to the specified address range.
    ///
    /// If the requested address range could not be accessed (e.g: due to
    /// MMU protection, unhanded page fault, etc...), return `Ok(false)` to
    /// signal that the requested memory could not be written to.
    ///
    /// As a reminder, `Err(Self::Error)` should only be returned if a memory
    /// write results in a **fatal** target error.
    fn write_addrs(
        &mut self,
        start_addr: <Self::Arch as Arch>::Usize,
        data: &[u8],
        tid: Tid,
    ) -> Result<bool, Self::Error>;

    /// List all currently active threads.
    ///
    /// See [the section above](#bare-metal-targets) on implementing
    /// thread-related methods on bare-metal (threadless) targets.
    fn list_active_threads(
        &mut self,
        thread_is_active: &mut dyn FnMut(Tid),
    ) -> Result<(), Self::Error>;

    /// Check if the specified thread is alive.
    ///
    /// As a convenience, this method provides a default implementation which
    /// uses `list_active_threads` to do a linear-search through all active
    /// threads. On thread-heavy systems, it may be more efficient
    /// to override this method with a more direct query.
    fn is_thread_alive(&mut self, tid: Tid) -> Result<bool, Self::Error> {
        let mut found = false;
        self.list_active_threads(&mut |active_tid| {
            if tid == active_tid {
                found = true;
            }
        })?;
        Ok(found)
    }
}

/// Describes why a thread stopped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ThreadStopReason<U> {
    /// Completed the single-step request.
    DoneStep,
    /// `check_gdb_interrupt` returned `true`
    GdbInterrupt,
    /// Halted
    Halted,
    /// A thread hit a software breakpoint (e.g. due to a trap instruction).
    ///
    /// NOTE: This does not necessarily have to be a breakpoint configured by
    /// the client/user of the current GDB session.
    SwBreak(Tid),
    /// A thread hit a hardware breakpoint.
    HwBreak(Tid),
    /// A thread hit a watchpoint.
    Watch {
        /// Which thread hit the watchpoint
        tid: Tid,
        /// Kind of watchpoint that was hit
        kind: WatchKind,
        /// Address of watched memory
        addr: U,
    },
    /// The program received a signal
    Signal(u8),
}

/// An iterator of `(TidSelector, ResumeAction)` used to specify how threads
/// should be resumed when running in multi threaded mode. It is _guaranteed_ to
/// contain at least one action.
///
/// See the documentation for
/// [`Target::resume`](trait.Target.html#tymethod.resume) for more details.
pub struct Actions<'a> {
    inner: &'a mut dyn Iterator<Item = (TidSelector, ResumeAction)>,
}

impl core::fmt::Debug for Actions<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Actions {{ .. }}")
    }
}

impl Actions<'_> {
    pub(crate) fn new(iter: &mut dyn Iterator<Item = (TidSelector, ResumeAction)>) -> Actions<'_> {
        Actions { inner: iter }
    }
}

impl Iterator for Actions<'_> {
    type Item = (TidSelector, ResumeAction);
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}
