use core::fmt::Debug;

use crate::internal::*;
use crate::{arch::Arch, ConsoleOutput, OptResult, Tid, TidSelector, SINGLE_THREAD_TID};

/// A collection of methods and metadata a `GdbStub` can use to control and
/// debug a system.
///
/// There are several [provided methods](#provided-methods) that can optionally
/// be implemented to enable additional advanced GDB debugging functionality.
///
/// ### Handling Threads on Bare-Metal Hardware
///
/// On bare-metal targets (such as microcontrollers or emulators), it's common
/// to treat individual _CPU cores_ as a separate "threads". e.g: in a dual-core
/// system, [CPU0, CPU1] might be mapped to [TID1, TID2] (note that TIDs cannot
/// be zero).
///
/// ### What's with the `<Self::Arch as Arch>::` syntax?
///
/// Yeah, sorry about that!
///
/// If [rust-lang/rust#38078](https://github.com/rust-lang/rust/issues/38078)
/// every gets fixed, `<Self::Arch as Arch>::Foo` will be simplified to just
/// `Self::Arch::Foo`.
///
/// Until then, when implementing `Target`, it's recommended to use the concrete
/// type directly. e.g: on a 32-bit platform, instead of writing `<Self::Arch
/// as Arch>::Usize`, use `u32` directly.
#[allow(clippy::type_complexity)]
pub trait Target {
    /// The target's architecture.
    type Arch: Arch;

    /// A target-specific **fatal** error.
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
    /// # Kinds of Targets
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
    /// Lastly, when returning a `(Tid, StopReason)` pair, the
    /// [`gdbstub::SINGLE_THREAD_TID`](constant.SINGLE_THREAD_TID.html) constant
    /// should be used for the `Tid` field.
    ///
    /// ### Multi-Threaded Targets
    ///
    /// If a Target ever lists more than one thread as active in
    /// `list_active_threads`, `gdbstub` switches to multithreaded mode. In
    /// this mode, the `actions` iterator may return more than one `action`.
    ///
    /// At the moment, `gdbstub` only supports GDB's
    /// ["All-Stop" mode](https://sourceware.org/gdb/current/onlinedocs/gdb/All_002dStop-Mode.html),
    /// whereby _all_ threads should be stopped when returning from `resume`
    /// (not just the thread responsible for the `StopReason`).
    ///
    /// ### Bare-Metal Targets
    ///
    /// See [the section above](#handling-threads-on-bare-metal-hardware) on how
    /// to use "threads" on bare-metal (threadless) targets to debug
    /// individual CPU cores.
    fn resume(
        &mut self,
        actions: Actions<'_>,
        check_gdb_interrupt: &mut dyn FnMut() -> bool,
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
    ///
    /// ### Handling non-fatal invalid memory reads
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
    ) -> Result<bool, Self::Error>;

    /// Write bytes to the specified address range.
    ///
    /// ### Handling non-fatal invalid memory writes
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
    ) -> Result<bool, Self::Error>;

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
    ) -> OptResult<bool, Self::Error> {
        let _ = (addr, op);
        Err(MaybeUnimpl::no_impl())
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
    ) -> OptResult<bool, Self::Error> {
        let _ = (addr, op, kind);
        Err(MaybeUnimpl::no_impl())
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
    /// Intermediate console output can be written back to the GDB client using
    /// the provided `ConsoleOutput` object + the
    /// [`gdbstub::output!`](macro.output.html) macro.
    ///
    /// _Note:_ The maximum length of incoming commands is limited by the size
    /// of the packet buffer provided to the [`GdbStub`](struct.GdbStub.html).
    /// Specifically, commands can only be up to `(buf.len() - 10) / 2` bytes.
    fn handle_monitor_cmd(
        &mut self,
        cmd: &[u8],
        out: ConsoleOutput<'_>,
    ) -> OptResult<(), Self::Error> {
        let _ = (cmd, out);
        Err(MaybeUnimpl::no_impl())
    }

    /// (optional|multithreading) List all currently active threads.
    ///
    /// See [the section above](#handling-threads-on-bare-metal-hardware) on
    /// implementing thread-related methods on bare-metal (threadless) targets.
    fn list_active_threads(
        &mut self,
        thread_is_active: &mut dyn FnMut(Tid),
    ) -> Result<(), Self::Error> {
        thread_is_active(SINGLE_THREAD_TID);
        Ok(())
    }

    /// (optional|multithreading) Select a specific thread to perform subsequent
    /// operations on (e.g: read/write registers, access memory, etc...)
    ///
    /// This method **must** be implemented if `list_active_threads` ever
    /// returns more than one thread!
    fn set_current_thread(&mut self, tid: Tid) -> OptResult<(), Self::Error> {
        let _ = tid;
        Err(MaybeUnimpl::no_impl())
    }

    /// (optional|multithreading) Check if the specified thread is alive.
    ///
    /// As a convenience, this method provides a default implementation which
    /// uses `list_active_threads` to do a linear-search through all active
    /// threads. On thread-heavy systems, it may be more efficient
    /// to override this method with a more direct query.
    fn is_thread_alive(&mut self, tid: Tid) -> OptResult<bool, Self::Error> {
        let mut found = false;
        self.list_active_threads(&mut |active_tid| {
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

/// An iterator of `(TidSelector, ResumeAction)`, used to specify how particular
/// threads should be resumed. It is _guaranteed_ to contain at least one
/// action.
///
/// See the documentation for
/// [`Target::resume`](trait.Target.html#tymethod.resume) for more details.
pub struct Actions<'a> {
    inner: &'a mut dyn Iterator<Item = (TidSelector, ResumeAction)>,
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

macro_rules! impl_dyn_target {
    ($type:ty) => {
        #[allow(clippy::type_complexity)]
        impl<A, E> Target for $type
        where
            A: Arch,
        {
            type Arch = A;
            type Error = E;

            fn resume(
                &mut self,
                actions: Actions<'_>,
                check_gdb_interrupt: &mut dyn FnMut() -> bool,
            ) -> Result<(Tid, StopReason<<Self::Arch as Arch>::Usize>), Self::Error> {
                (**self).resume(actions, check_gdb_interrupt)
            }

            fn read_registers(
                &mut self,
                regs: &mut <Self::Arch as Arch>::Registers,
            ) -> Result<(), Self::Error> {
                (**self).read_registers(regs)
            }

            fn write_registers(
                &mut self,
                regs: &<Self::Arch as Arch>::Registers,
            ) -> Result<(), Self::Error> {
                (**self).write_registers(regs)
            }

            fn read_addrs(
                &mut self,
                start_addr: <Self::Arch as Arch>::Usize,
                data: &mut [u8],
            ) -> Result<bool, Self::Error> {
                (**self).read_addrs(start_addr, data)
            }

            fn write_addrs(
                &mut self,
                start_addr: <Self::Arch as Arch>::Usize,
                data: &[u8],
            ) -> Result<bool, Self::Error> {
                (**self).write_addrs(start_addr, data)
            }

            fn update_sw_breakpoint(
                &mut self,
                addr: <Self::Arch as Arch>::Usize,
                op: BreakOp,
            ) -> Result<bool, Self::Error> {
                (**self).update_sw_breakpoint(addr, op)
            }

            fn update_hw_breakpoint(
                &mut self,
                addr: <Self::Arch as Arch>::Usize,
                op: BreakOp,
            ) -> OptResult<bool, Self::Error> {
                (**self).update_hw_breakpoint(addr, op)
            }

            fn update_hw_watchpoint(
                &mut self,
                addr: <Self::Arch as Arch>::Usize,
                op: BreakOp,
                kind: WatchKind,
            ) -> OptResult<bool, Self::Error> {
                (**self).update_hw_watchpoint(addr, op, kind)
            }

            fn handle_monitor_cmd(
                &mut self,
                cmd: &[u8],
                out: ConsoleOutput<'_>,
            ) -> OptResult<(), Self::Error> {
                (**self).handle_monitor_cmd(cmd, out)
            }

            fn list_active_threads(
                &mut self,
                thread_is_active: &mut dyn FnMut(Tid),
            ) -> Result<(), Self::Error> {
                (**self).list_active_threads(thread_is_active)
            }

            fn set_current_thread(&mut self, tid: Tid) -> OptResult<(), Self::Error> {
                (**self).set_current_thread(tid)
            }

            fn is_thread_alive(&mut self, tid: Tid) -> OptResult<bool, Self::Error> {
                (**self).is_thread_alive(tid)
            }
        }
    };
}

impl_dyn_target!(&mut dyn Target<Arch = A, Error = E>);
#[cfg(feature = "alloc")]
impl_dyn_target!(alloc::boxed::Box<dyn Target<Arch = A, Error = E>>);
