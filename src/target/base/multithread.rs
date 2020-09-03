use crate::arch::{Arch, Registers};
use crate::target::base::*;
use crate::target::Target;

/// Base debugging operations for multi threaded targets
#[allow(clippy::type_complexity)]
pub trait MultiThread: Target {
    /// Resume execution on the target.
    ///
    /// `actions` specifies how various threads should be resumed (i.e:
    /// single-step vs. resume). It is _guaranteed_ to contain at least one
    /// action.
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
    /// (not just the thread responsible for the `StopReason`).
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
    ) -> Result<(Tid, StopReason<<Self::Arch as Arch>::Usize>), Self::Error>;

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
    /// _Note:_ Reading/writing individual registers is a relatively recent
    /// addition to `gdbstub`, and as such, there are still several built-in
    /// `arch` implementations which have not been updated with a valid `RegId`
    /// type. Instead, the use the default unit `()` type.
    // FIXME: should change default RegId to `usize`, to at least allow users to
    // implement the feature manually if a friendly `RegId` hasn't been defined yet!
    fn read_register(
        &mut self,
        reg_id: <<Self::Arch as Arch>::Registers as Registers>::RegId,
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
    /// _Note:_ Reading/writing individual registers is a relatively recent
    /// addition to `gdbstub`, and as such, there are still several built-in
    /// `arch` implementations which have not been updated with a valid `RegId`
    /// type. Instead, the use the default unit `()` type.
    // FIXME: should change default RegId to `usize`, to at least allow users to
    // implement the feature manually if a friendly `RegId` hasn't been defined yet!
    fn write_register(
        &mut self,
        reg_id: <<Self::Arch as Arch>::Registers as Registers>::RegId,
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
    /// See [the section above](#handling-threads-on-bare-metal-hardware) on
    /// implementing thread-related methods on bare-metal (threadless) targets.
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
