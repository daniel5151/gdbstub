use core::fmt::Debug;
use core::ops::Range;

use crate::{Arch, Tid, TidSelector, SINGLE_THREAD_TID};

/// A collection of methods and metadata a `GdbStub` can use to control and
/// debug a system.
///
/// There are several [provided methods](#provided-methods) that can optionally
/// be implemented to enable additional advanced GDB debugging functionality.
///
/// ### Handling Threads on Bare-Metal Hardware
///
/// On bare-metal targets, it's common to treat individual _CPU cores_ as a
/// separate "threads". e.g: in a dual-core system, [CPU0, CPU1] might be mapped
/// to [TID1, TID2] (note that TIDs cannot be zero).
///
/// ### What's with the `<Self::Arch as Arch>::` syntax?
///
/// Yeah, sorry about that!
///
/// If [rust-lang/rust#38078](https://github.com/rust-lang/rust/issues/38078)
/// every gets fixed, `<Self::Arch as Arch>::Usize` can be simplified to the
/// much more readable `Self::Arch::Usize`.
///
/// Until then, when implementing `Target`, I recommend using the concrete
/// type directly. (e.g: on a 32-bit platform, instead of writing `<Self::Arch
/// as Arch>::Usize`, just use `u32` directly)
#[allow(clippy::type_complexity)]
pub trait Target {
    /// The target's architecture. If that target's architecture isn't listed
    /// under `gdbstub::arch`, it's straightforward to define a custom `Arch`.
    ///
    /// _Author's Note:_ If you end up implementing a missing `Arch`
    /// implementation, please consider upstreaming it's implementation!
    type Arch: Arch;

    /// A target-specific fatal error.
    type Error;

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
    /// ### Single-Threaded Targets
    ///
    /// For single-threaded Target's (i.e: those that have not implemented any
    /// (optional|multithreading) features), it's safe to ignore the
    /// `TidSelector` component of the `actions` iterator entirely. Moreover,
    /// it's safe to assume that there will only ever be a single `action`
    /// returned by the `actions` iterator. As such, the following snippet
    /// should never panic:
    ///
    /// `let (_, action) = actions.next().unwrap();`
    ///
    /// Lastly, When returning a `(Tid, StopReason)` pair, use the provided
    /// [`gdbstub::SINGLE_THREAD_TID`](constant.SINGLE_THREAD_TID.html) constant
    /// for the `Tid` field.
    ///
    /// ### Multi-Threaded Targets
    ///
    /// If a Target ever lists more than one thread as active in
    /// `list_active_threads`, the GdbStub switches to multithreaded mode. In
    /// this mode, the `actions` iterator may return more than one `action`.
    ///
    /// At the moment, `gdbstub` only supports GDB's
    /// ["All-Stop" mode](https://sourceware.org/gdb/current/onlinedocs/gdb/All_002dStop-Mode.html),
    /// whereby _all_ threads should be stopped prior upon returning from
    /// `resume`, not just the thread responsible for the `StopReason`.
    ///
    /// ### Bare-Metal Targets
    ///
    /// See [the section above](#handling-threads-on-bare-metal-hardware) on how
    /// to use "threads" on bare-metal (threadless) targets to debug
    /// individual CPU cores.
    fn resume(
        &mut self,
        actions: impl Iterator<Item = (TidSelector, ResumeAction)>,
        check_gdb_interrupt: impl FnMut() -> bool,
    ) -> Result<(Tid, StopReason<<Self::Arch as Arch>::Usize>), Self::Error>;

    /// Read the target's registers.
    ///
    /// On multi-threaded systems, this method **must** respect the currently
    /// selected thread (set via the `set_current_thread` method).
    fn read_registers(
        &mut self,
        regs: &mut <Self::Arch as Arch>::Registers,
    ) -> Result<(), Self::Error>;

    /// Write the target's registers.
    ///
    /// On multi-threaded systems, this method **must** respect the currently
    /// selected thread (set via the `set_current_thread` method).
    fn write_registers(
        &mut self,
        regs: &<Self::Arch as Arch>::Registers,
    ) -> Result<(), Self::Error>;

    /// Read bytes from the specified address range.
    fn read_addrs(
        &mut self,
        addrs: Range<<Self::Arch as Arch>::Usize>,
        val: impl FnMut(u8),
    ) -> Result<(), Self::Error>;

    /// Write bytes to the specified address range.
    fn write_addrs(
        &mut self,
        start_addr: <Self::Arch as Arch>::Usize,
        data: &[u8],
    ) -> Result<(), Self::Error>;

    /// Set/remove a software breakpoint.
    /// Return `Ok(false)` if the operation could not be completed.
    ///
    /// See [this stackoverflow discussion](https://stackoverflow.com/questions/8878716/what-is-the-difference-between-hardware-and-software-breakpoints)
    /// about the differences between hardware and software breakpoints.
    ///
    /// _Author's recommendation:_ If you're implementing `Target` for an
    /// emulator using an _interpreted_ CPU (as opposed to a JIT), the
    /// simplest way to implement "software" breakpoints is to check the
    /// `PC` value after each CPU cycle.
    fn update_sw_breakpoint(
        &mut self,
        addr: <Self::Arch as Arch>::Usize,
        op: BreakOp,
    ) -> Result<bool, Self::Error>;

    /// (optional) Set/remove a hardware breakpoint.
    /// Return `Ok(false)` if the operation could not be completed.
    ///
    /// See [this stackoverflow discussion](https://stackoverflow.com/questions/8878716/what-is-the-difference-between-hardware-and-software-breakpoints)
    /// about the differences between hardware and software breakpoints.
    ///
    /// _Author's recommendation:_ If you're implementing `Target` for an
    /// emulator using an _interpreted_ CPU (as opposed to a JIT), there
    /// shouldn't be any reason to implement this method (as software
    /// breakpoints are likely to be just-as-fast).
    fn update_hw_breakpoint(
        &mut self,
        addr: <Self::Arch as Arch>::Usize,
        op: BreakOp,
    ) -> Option<Result<bool, Self::Error>> {
        let _ = (addr, op);
        None
    }

    /// (optional) Set/remove a hardware watchpoint.
    /// Return `Ok(false)` if the operation could not be completed.
    ///
    /// See the [GDB documentation](https://sourceware.org/gdb/current/onlinedocs/gdb/Set-Watchpoints.html)
    /// regarding watchpoints for how they're supposed to work.
    ///
    /// _NOTE:_ If this method isn't implemented, GDB will default to using
    /// _software watchpoints_, which tend to be excruciatingly slow (as
    /// they are implemented by single-stepping the system, and reading the
    /// watched memory location after each step).
    fn update_hw_watchpoint(
        &mut self,
        addr: <Self::Arch as Arch>::Usize,
        op: BreakOp,
        kind: WatchKind,
    ) -> Option<Result<bool, Self::Error>> {
        let _ = (addr, op, kind);
        None
    }

    /// (optional) Handle custom commands sent using the `monitor` command.
    ///
    /// The GDB remote serial protocol includes a built-in mechanism to send
    /// arbitrary commands to the remote stub: the `monitor` command. For
    /// example, running `monitor dbg` from the GDB client will invoke
    /// `handle_monitor_cmd` with `cmd = b"dbg"`.
    ///
    /// Commands are _not_ guaranteed to be valid UTF-8, hence the use of
    /// `&[u8]` as opposed to `&str`.
    ///
    /// Output can be written back to the GDB client using the provided `output`
    /// callback.
    ///
    /// _Note:_ Sending a single large output message is preferable to sending
    /// multiple smaller output messages, as the `output` callback does not
    /// provide any form of IO buffering. Each call to `output` will send a new
    /// GDB packet over the `Connection`.
    ///
    /// _Note:_ The maximum length of incoming commands is dependent on the
    /// length of the packet buffer used by [`GdbStub`](struct.GdbStub.html),
    /// determined by the formula `(buf.len() - 10) / 2`.
    fn handle_monitor_cmd(
        &mut self,
        cmd: &[u8],
        output: impl FnMut(&[u8]),
    ) -> Result<Option<()>, Self::Error> {
        let _ = (cmd, output);
        Ok(None)
    }

    /// (optional|multithreading) List all currently active threads.
    ///
    /// See [the section above](#handling-threads-on-bare-metal-hardware) on
    /// implementing thread-related methods on bare-metal (threadless) targets.
    fn list_active_threads(
        &mut self,
        mut thread_is_active: impl FnMut(Tid),
    ) -> Result<(), Self::Error> {
        thread_is_active(SINGLE_THREAD_TID);
        Ok(())
    }

    /// (optional|multithreading) Select a specific thread to perform subsequent
    /// operations on (e.g: read/write registers, access memory, etc...)
    ///
    /// This method **must** be implemented if `list_active_threads` ever
    /// returns more than one thread!
    fn set_current_thread(&mut self, tid: Tid) -> Option<Result<(), Self::Error>> {
        let _ = tid;
        None
    }

    /// (optional|multithreading) Check if the specified thread is alive.
    ///
    /// As a convenience, this method provides a default implementation which
    /// uses `list_active_threads` to do a linear-search through all active
    /// threads. On thread-heavy systems, it may be more efficient
    /// to override this method with a more direct query.
    fn is_thread_alive(&mut self, tid: Tid) -> Result<bool, Self::Error> {
        let mut found = false;
        self.list_active_threads(|active_tid| {
            if tid == active_tid {
                found = true;
            }
        })?;
        Ok(found)
    }
}

/// The kind of watchpoint that should be set/removed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WatchKind {
    /// Fire when the memory location is written to.
    Write,
    /// Fire when the memory location is read from.
    Read,
    /// Fire when the memory location is written to and/or read from.
    ReadWrite,
}

/// Add / Remove a breakpoint / watchpoint
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BreakOp {
    /// Add a new breakpoint / watchpoint.
    Add,
    /// Remove an existing breakpoint / watchpoint.
    Remove,
}

/// Describes why the target stopped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
}

/// Describes how the target should resume the specified thread.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResumeAction {
    /// Continue execution (until the next event occurs).
    Continue,
    /// Step forward a single instruction.
    Step,
    /* ContinueWithSignal(u8),
     * StepWithSignal(u8),
     * Stop,
     * StepInRange(core::ops::Range<U>), */
}
