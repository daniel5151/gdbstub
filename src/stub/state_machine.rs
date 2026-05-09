//! Low-level state-machine interface that underpins [`GdbStub`].
//
// TODO: write some proper documentation + examples of how to interface with
// this API.
//!
//! # Hey, what gives? Where are all the docs!?
//!
//! Yep, sorry about that!
//!
//! `gdbstub` 0.6 turned out ot be a pretty massive release, and documenting
//! everything has proven to be a somewhat gargantuan task that's kept delaying
//! the release data further and further back...
//!
//! To avoid blocking the release any further, I've decided to leave this bit of
//! the API sparsely documented.
//!
//! If you're interested in using this API directly (e.g: to integrate `gdbstub`
//! into a `no_std` project, or to use `gdbstub` in a non-blocking manner
//! alongside `async/await` / a project specific event loop), your best bet
//! would be to review the following bits of code to get a feel for the API:
//!
//! - The implementation of [`GdbStub::run_blocking`]
//! - Implementations of [`BlockingEventLoop`] used alongside
//!   `GdbStub::run_blocking` (e.g: the in-tree `armv4t` / `armv4t_multicore`
//!   examples)
//! - Real-world projects using the API (see the repo's README.md)
//!
//! If you have any questions, feel free to open a discussion thread over at the
//! [`gdbstub` GitHub repo](https://github.com/daniel5151/gdbstub/).
//!
//! [`BlockingEventLoop`]: super::run_blocking::BlockingEventLoop
//! [`GdbStub::run_blocking`]: super::GdbStub::run_blocking

use super::core_impl::GdbStubImpl;
use super::core_impl::State;
use super::DisconnectReason;
use super::GdbStub;
use crate::arch::Arch;
use crate::arch::RegId;
use crate::common::Signal;
use crate::conn::Connection;
use crate::protocol::recv_packet::RecvPacketStateMachine;
use crate::protocol::Packet;
use crate::protocol::ResponseWriter;
use crate::protocol::ResponseWriterState;
use crate::stub::error::GdbStubError;
use crate::stub::error::InternalError;
use crate::target::ext::base::reverse_exec::ReplayLogPosition;
use crate::target::ext::breakpoints::WatchKind;
use crate::target::ext::catch_syscalls::CatchSyscallPosition;
use crate::target::Target;
use managed::ManagedSlice;

/// State-machine interface to `GdbStub`.
///
/// See the [module level documentation](self) for more details.
pub enum GdbStubStateMachine<'a, T, C>
where
    T: Target,
    C: Connection,
{
    /// The target is completely stopped, and the GDB stub is waiting for
    /// additional input.
    Idle(GdbStubStateMachineInner<'a, state::Idle, T, C>),
    /// The target is currently running, and the GDB client is waiting for
    /// the target to report a stop reason.
    ///
    /// Note that the client may still send packets to the target
    /// (e.g: to trigger a Ctrl-C interrupt).
    Running(GdbStubStateMachineInner<'a, state::Running, T, C>),
    /// The GDB client has sent a Ctrl-C interrupt to the target.
    CtrlCInterrupt(GdbStubStateMachineInner<'a, state::CtrlCInterrupt, T, C>),
    /// The GDB client has disconnected.
    Disconnected(GdbStubStateMachineInner<'a, state::Disconnected, T, C>),
}

/// State machine typestates.
///
/// The types in this module are used to parameterize instances of
/// [`GdbStubStateMachineInner`], thereby enforcing that certain API methods
/// can only be called while the stub is in a certain state.
// As an internal implementation detail, they _also_ carry state-specific
// payloads, which are used when transitioning between states.
pub mod state {
    use super::*;

    // used internally when logging state transitions
    pub(crate) const MODULE_PATH: &str = concat!(module_path!(), "::");

    /// Typestate corresponding to the "Idle" state.
    #[non_exhaustive]
    pub struct Idle {}

    /// Typestate corresponding to the "Running" state.
    #[non_exhaustive]
    pub struct Running {}

    /// Typestate corresponding to the "CtrlCInterrupt" state.
    #[non_exhaustive]
    pub struct CtrlCInterrupt {
        pub(crate) from_idle: bool,
    }

    /// Typestate corresponding to the "Disconnected" state.
    #[non_exhaustive]
    pub struct Disconnected {
        pub(crate) reason: DisconnectReason,
    }
}

/// Internal helper macro to convert between a particular inner state into
/// its corresponding `GdbStubStateMachine` variant.
macro_rules! impl_from_inner {
        ($state:ident $($tt:tt)*) => {
            impl<'a, T, C> From<GdbStubStateMachineInner<'a, state::$state $($tt)*, T, C>>
                for GdbStubStateMachine<'a, T, C>
            where
                T: Target,
                C: Connection,
            {
                fn from(inner: GdbStubStateMachineInner<'a, state::$state $($tt)*, T, C>) -> Self {
                    GdbStubStateMachine::$state(inner)
                }
            }
        };
    }

impl_from_inner!(Idle);
impl_from_inner!(Running);
impl_from_inner!(CtrlCInterrupt);
impl_from_inner!(Disconnected);

/// Internal helper trait to cut down on boilerplate required to transition
/// between states.
trait Transition<'a, T, C>
where
    T: Target,
    C: Connection,
{
    /// Transition between different state machine states
    fn transition<S2>(self, state: S2) -> GdbStubStateMachineInner<'a, S2, T, C>;
}

impl<'a, S1, T, C> Transition<'a, T, C> for GdbStubStateMachineInner<'a, S1, T, C>
where
    T: Target,
    C: Connection,
{
    #[inline(always)]
    fn transition<S2>(self, state: S2) -> GdbStubStateMachineInner<'a, S2, T, C> {
        if log::log_enabled!(log::Level::Trace) {
            let s1 = core::any::type_name::<S1>();
            let s2 = core::any::type_name::<S2>();
            log::trace!(
                "transition: {:?} --> {:?}",
                s1.strip_prefix(state::MODULE_PATH).unwrap_or(s1),
                s2.strip_prefix(state::MODULE_PATH).unwrap_or(s2)
            );
        }
        GdbStubStateMachineInner { i: self.i, state }
    }
}

// split off `GdbStubStateMachineInner`'s non state-dependant data into separate
// struct for code bloat optimization (i.e: `transition` will generate better
// code when the struct is cleaved this way).
struct GdbStubStateMachineReallyInner<'a, T: Target, C: Connection> {
    conn: C,
    packet_buffer: ManagedSlice<'a, u8>,
    recv_packet: RecvPacketStateMachine,
    inner: GdbStubImpl<T, C>,
}

/// Core state machine implementation that is parameterized by various
/// [states](state). Can be converted back into the appropriate
/// [`GdbStubStateMachine`] variant via [`Into::into`].
pub struct GdbStubStateMachineInner<'a, S, T: Target, C: Connection> {
    i: GdbStubStateMachineReallyInner<'a, T, C>,
    state: S,
}

/// Methods which can be called regardless of the current state.
impl<'a, S, T, C> GdbStubStateMachineInner<'a, S, T, C>
where
    T: Target,
    C: Connection,
{
    /// Return a mutable reference to the underlying connection.
    pub fn borrow_conn(&mut self) -> &mut C {
        &mut self.i.conn
    }
}

/// Methods which can only be called from the [`GdbStubStateMachine::Idle`]
/// state.
impl<'a, T, C> GdbStubStateMachineInner<'a, state::Idle, T, C>
where
    T: Target,
    C: Connection,
{
    /// Internal entrypoint into the state machine.
    pub(crate) fn from_plain_gdbstub(
        stub: GdbStub<'a, T, C>,
    ) -> GdbStubStateMachineInner<'a, state::Idle, T, C> {
        GdbStubStateMachineInner {
            i: GdbStubStateMachineReallyInner {
                conn: stub.conn,
                packet_buffer: stub.packet_buffer,
                recv_packet: RecvPacketStateMachine::new(),
                inner: stub.inner,
            },
            state: state::Idle {},
        }
    }

    /// Pass a byte to the GDB stub.
    pub fn incoming_data(
        mut self,
        target: &mut T,
        byte: u8,
    ) -> Result<GdbStubStateMachine<'a, T, C>, GdbStubError<T::Error, C::Error>> {
        let packet_buffer = match self.i.recv_packet.pump(&mut self.i.packet_buffer, byte)? {
            Some(buf) => buf,
            None => return Ok(self.into()),
        };

        let packet = Packet::from_buf(target, packet_buffer).map_err(InternalError::PacketParse)?;
        let state = self
            .i
            .inner
            .handle_packet(target, &mut self.i.conn, packet)?;
        Ok(match state {
            State::Pump => self.into(),
            State::Disconnect(reason) => self.transition(state::Disconnected { reason }).into(),
            State::DoResume => self.transition(state::Running {}).into(),
            State::CtrlCInterrupt => self
                .transition(state::CtrlCInterrupt { from_idle: true })
                .into(),
        })
    }
}

/// A helper that enforces protocol-level constraints related to reporting
/// stop-reasons.
pub struct StopReasonReporter<'a, 't, T, C, const USING_T_PACKET: bool, const CAN_ADD_CORE: bool>
where
    T: Target,
    C: Connection,
{
    target: &'t mut T,
    res: ResponseWriterState,
    gdb: GdbStubStateMachineInner<'a, state::Running, T, C>,
}

impl<'a, 't, T, C, const USING_T_PACKET: bool, const CAN_ADD_CORE: bool>
    StopReasonReporter<'a, 't, T, C, USING_T_PACKET, CAN_ADD_CORE>
where
    T: Target,
    C: Connection,
{
    /// Obtain a mutable handle to the `target`
    pub fn borrow_target<'b: 't>(&'b mut self) -> &'t T {
        self.target
    }
}

impl<'a, 't, T, C, const CAN_ADD_CORE: bool> StopReasonReporter<'a, 't, T, C, true, CAN_ADD_CORE>
where
    T: Target,
    C: Connection,
{
    /// Finalize the stop reply packet, and transition the state machine into
    /// the Idle state.
    pub fn done(
        mut self,
    ) -> Result<GdbStubStateMachine<'a, T, C>, GdbStubError<T::Error, C::Error>> {
        ResponseWriter::from_state(&mut self.gdb.i.conn, self.res)
            .flush()
            .map_err(InternalError::from)?;
        Ok(self.gdb.transition(state::Idle {}).into())
    }

    /// Include an expedited register value in the stop reply packet.
    ///
    /// Including these registers is entirely optional*, but can be a useful way
    /// to reduce round trip latency.
    ///
    /// **Note:** In order to use this method, the Target's [`Arch`]
    /// implementation MUST implement a valid [`RegId::to_raw_id`]
    /// implementation. If the method is unimplemented, `gdbstub` will report an
    /// error.
    ///
    /// The iterator yields `(register_number, value_bytes)` pairs that are
    /// written as expedited registers in the T-packet. Values should be in
    /// target byte order (typically little-endian).
    ///
    /// \* Though there are known instances where a particular client + arch
    /// combo may _require_ including one or more registers inline with the stop
    /// reply packet. e.g: WASM on LLDB requires reporting the PC.
    ///
    /// [`RegId::to_raw_id`]: crate::arch::RegId::to_raw_id
    pub fn add_reg(
        self,
        reg_id: <<T as Target>::Arch as Arch>::RegId,
        value: &[u8],
    ) -> Result<Self, GdbStubError<T::Error, C::Error>> {
        let Self {
            target,
            res,
            mut gdb,
        } = self;

        let mut res = ResponseWriter::from_state(&mut gdb.i.conn, res);

        let reg = reg_id.to_raw_id().ok_or(InternalError::MissingToRawId)?;
        res.write_num(reg).map_err(InternalError::from)?;
        res.write_str(":").map_err(InternalError::from)?;
        res.write_hex_buf(value).map_err(InternalError::from)?;
        res.write_str(";").map_err(InternalError::from)?;

        Ok(Self {
            target,
            res: res.into_state(),
            gdb,
        })
    }
}

impl<'a, 't, T, C> StopReasonReporter<'a, 't, T, C, true, true>
where
    T: Target,
    C: Connection,
{
    /// Include metadata about what "core" the stop event was detected on.
    ///
    /// This can only be called once.
    pub fn add_core(
        self,
        core: usize,
    ) -> Result<StopReasonReporter<'a, 't, T, C, true, false>, GdbStubError<T::Error, C::Error>>
    {
        let Self {
            target,
            res,
            mut gdb,
        } = self;

        let mut res = ResponseWriter::from_state(&mut gdb.i.conn, res);

        res.write_str("core:").map_err(InternalError::from)?;
        res.write_num(core).map_err(InternalError::from)?;
        res.write_str(";").map_err(InternalError::from)?;

        Ok(StopReasonReporter {
            target,
            res: res.into_state(),
            gdb,
        })
    }
}

// DEVNOTE: for certain stop reasons, it's possible to add a `Target` bound that
// restricts calling the stop reason method entirely if the corresponding IDET
// hasn't been implemented.
//
// This is a nice UX trick, but it is _not_ a bulletproof way to enforce
// protocol invariants.
//
// i.e: a `Target` is well within it's right to _dynamically_ toggle an IDET on
// or off (via `supports_*`), which means downstream runtime checks (in the
// `finish_*` methods) are still required.
impl<'a, 't, T, C, const CAN_ADD_CORE: bool> StopReasonReporter<'a, 't, T, C, false, CAN_ADD_CORE>
where
    T: Target,
    C: Connection,
{
    // ---- Stop Reasons that DO NOT use the T packet ---- //

    /// Completed the single-step request.
    ///
    /// This stop reason immediately resolves, bypassing [`StopReasonReporter`]
    /// and transitioning the state machine into the Idle state.
    pub fn done_step(
        self,
    ) -> Result<GdbStubStateMachine<'a, T, C>, GdbStubError<T::Error, C::Error>> {
        let Self {
            target: _,
            res,
            mut gdb,
        } = self;

        let mut res = ResponseWriter::from_state(&mut gdb.i.conn, res);

        gdb.i.inner.finish_done_step(&mut res)?;
        res.flush().map_err(InternalError::from)?;

        Ok(gdb.transition(state::Idle {}).into())
    }

    /// The process terminated with the specified signal number.
    ///
    /// This stop reason immediately resolves, bypassing [`StopReasonReporter`]
    /// and transitioning the state machine into the Disconnected state.
    pub fn terminated(
        self,
        signal: Signal,
    ) -> Result<GdbStubStateMachine<'a, T, C>, GdbStubError<T::Error, C::Error>> {
        let Self {
            target: _,
            res,
            mut gdb,
        } = self;

        let mut res = ResponseWriter::from_state(&mut gdb.i.conn, res);

        gdb.i.inner.finish_terminated(&mut res, signal)?;
        res.flush().map_err(InternalError::from)?;

        Ok(gdb
            .transition(state::Disconnected {
                reason: DisconnectReason::TargetTerminated(signal),
            })
            .into())
    }

    /// The process exited with the specified exit status code.
    ///
    /// This stop reason immediately resolves, bypassing [`StopReasonReporter`]
    /// and transitioning the state machine into the Disconnected state.
    pub fn exited(
        self,
        code: u8,
    ) -> Result<GdbStubStateMachine<'a, T, C>, GdbStubError<T::Error, C::Error>> {
        let Self {
            target: _,
            res,
            mut gdb,
        } = self;

        let mut res = ResponseWriter::from_state(&mut gdb.i.conn, res);

        gdb.i.inner.finish_exited(&mut res, code)?;
        res.flush().map_err(InternalError::from)?;

        Ok(gdb
            .transition(state::Disconnected {
                reason: DisconnectReason::TargetExited(code),
            })
            .into())
    }

    /// The program received a signal.
    ///
    /// This stop reason immediately resolves, bypassing [`StopReasonReporter`]
    /// and transitioning the state machine into the Idle state.
    pub fn signal(
        self,
        signal: Signal,
    ) -> Result<GdbStubStateMachine<'a, T, C>, GdbStubError<T::Error, C::Error>> {
        let Self {
            target: _,
            res,
            mut gdb,
        } = self;

        let mut res = ResponseWriter::from_state(&mut gdb.i.conn, res);

        gdb.i.inner.finish_signal(&mut res, signal)?;
        res.flush().map_err(InternalError::from)?;

        Ok(gdb.transition(state::Idle {}).into())
    }

    // ---- Stop Reasons that DO use the T packet ---- //

    /// A specific thread received a signal.
    pub fn signal_with_thread(
        self,
        signal: Signal,
        tid: T::Tid,
    ) -> Result<StopReasonReporter<'a, 't, T, C, true, true>, GdbStubError<T::Error, C::Error>>
    {
        let Self {
            target,
            res,
            mut gdb,
        } = self;

        let mut res = ResponseWriter::from_state(&mut gdb.i.conn, res);

        gdb.i
            .inner
            .finish_signal_with_thread(&mut res, target, tid, signal)?;

        Ok(StopReasonReporter {
            target,
            res: res.into_state(),
            gdb,
        })
    }

    /// A thread hit a software breakpoint (e.g. due to a trap instruction).
    ///
    /// Requires: [`SwBreakpoint`].
    ///
    /// NOTE: This does not necessarily have to be a breakpoint configured by
    /// the client/user of the current GDB session.
    ///
    /// [`SwBreakpoint`]: crate::target::ext::breakpoints::SwBreakpoint
    pub fn swbreak(
        self,
        tid: T::Tid,
    ) -> Result<StopReasonReporter<'a, 't, T, C, true, true>, GdbStubError<T::Error, C::Error>>
    where
        // DEVNOTE: see DEVNOTE above (on the `impl` block itself) for info on this type bound
        T: crate::target::ext::breakpoints::SwBreakpoint,
    {
        let Self {
            target,
            res,
            mut gdb,
        } = self;

        let mut res = ResponseWriter::from_state(&mut gdb.i.conn, res);

        gdb.i.inner.finish_swbreak(&mut res, target, tid)?;

        Ok(StopReasonReporter {
            target,
            res: res.into_state(),
            gdb,
        })
    }

    /// A thread hit a hardware breakpoint.
    ///
    /// Requires: [`HwBreakpoint`].
    ///
    /// [`HwBreakpoint`]: crate::target::ext::breakpoints::HwBreakpoint
    pub fn hwbreak(
        self,
        tid: T::Tid,
    ) -> Result<StopReasonReporter<'a, 't, T, C, true, true>, GdbStubError<T::Error, C::Error>>
    where
        // DEVNOTE: see DEVNOTE above (on the `impl` block itself) for info on this type bound
        T: crate::target::ext::breakpoints::HwBreakpoint,
    {
        let Self {
            target,
            res,
            mut gdb,
        } = self;

        let mut res = ResponseWriter::from_state(&mut gdb.i.conn, res);

        gdb.i.inner.finish_hwbreak(&mut res, target, tid)?;

        Ok(StopReasonReporter {
            target,
            res: res.into_state(),
            gdb,
        })
    }

    /// A thread hit a watchpoint.
    ///
    /// Requires: [`HwWatchpoint`].
    ///
    /// [`HwWatchpoint`]: crate::target::ext::breakpoints::HwWatchpoint
    pub fn watch(
        self,
        tid: T::Tid,
        kind: WatchKind,
        addr: <<T as Target>::Arch as Arch>::Usize,
    ) -> Result<StopReasonReporter<'a, 't, T, C, true, true>, GdbStubError<T::Error, C::Error>>
    where
        // DEVNOTE: see DEVNOTE above (on the `impl` block itself) for info on this type bound
        T: crate::target::ext::breakpoints::HwWatchpoint,
    {
        let Self {
            target,
            res,
            mut gdb,
        } = self;

        let mut res = ResponseWriter::from_state(&mut gdb.i.conn, res);

        gdb.i
            .inner
            .finish_watch(&mut res, target, tid, kind, addr)?;

        Ok(StopReasonReporter {
            target,
            res: res.into_state(),
            gdb,
        })
    }

    /// The program has reached the end of the logged replay events.
    ///
    /// Requires: [`ReverseCont`] or [`ReverseStep`].
    ///
    /// This is used for GDB's reverse execution. When playing back a recording,
    /// you may hit the end of the buffer of recorded events, and as such no
    /// further execution can be done. This stop reason tells GDB that this has
    /// occurred.
    ///
    /// [`ReverseCont`]: crate::target::ext::base::reverse_exec::ReverseCont
    /// [`ReverseStep`]: crate::target::ext::base::reverse_exec::ReverseStep
    pub fn replay_log(
        self,
        tid: Option<T::Tid>,
        pos: ReplayLogPosition,
    ) -> Result<StopReasonReporter<'a, 't, T, C, true, true>, GdbStubError<T::Error, C::Error>>
    {
        let Self {
            target,
            res,
            mut gdb,
        } = self;

        let mut res = ResponseWriter::from_state(&mut gdb.i.conn, res);

        gdb.i.inner.finish_replay_log(&mut res, target, tid, pos)?;

        Ok(StopReasonReporter {
            target,
            res: res.into_state(),
            gdb,
        })
    }

    /// The program has reached a syscall entry or return location.
    ///
    /// Requires: [`CatchSyscalls`].
    ///
    /// [`CatchSyscalls`]: crate::target::ext::catch_syscalls::CatchSyscalls
    pub fn catch_syscall(
        self,
        tid: Option<T::Tid>,
        number: <<T as Target>::Arch as Arch>::Usize,
        position: CatchSyscallPosition,
    ) -> Result<StopReasonReporter<'a, 't, T, C, true, true>, GdbStubError<T::Error, C::Error>>
    where
        // DEVNOTE: see DEVNOTE above (on the `impl` block itself) for info on this type bound
        T: crate::target::ext::catch_syscalls::CatchSyscalls,
    {
        let Self {
            target,
            res,
            mut gdb,
        } = self;

        let mut res = ResponseWriter::from_state(&mut gdb.i.conn, res);

        gdb.i
            .inner
            .finish_catch_syscall(&mut res, target, tid, number, position)?;

        Ok(StopReasonReporter {
            target,
            res: res.into_state(),
            gdb,
        })
    }

    /// A thread hit a specific library event.
    ///
    /// This stop reason indicates that loaded libraries have changed. The
    /// debugger should fetch a new list of loaded libraries.
    pub fn library(
        self,
        tid: T::Tid,
    ) -> Result<StopReasonReporter<'a, 't, T, C, true, true>, GdbStubError<T::Error, C::Error>>
    {
        let Self {
            target,
            res,
            mut gdb,
        } = self;

        let mut res = ResponseWriter::from_state(&mut gdb.i.conn, res);

        gdb.i.inner.finish_library(&mut res, target, tid)?;

        Ok(StopReasonReporter {
            target,
            res: res.into_state(),
            gdb,
        })
    }

    /// A thread created a new process via fork.
    ///
    /// This indicates that a fork system call was executed, creating a new
    /// child process.
    ///
    /// Requires: [`Target::use_fork_stop_reason`].
    pub fn fork(
        self,
        cur_tid: T::Tid,
        new_tid: T::Tid,
    ) -> Result<StopReasonReporter<'a, 't, T, C, true, true>, GdbStubError<T::Error, C::Error>>
    {
        let Self {
            target,
            res,
            mut gdb,
        } = self;

        let mut res = ResponseWriter::from_state(&mut gdb.i.conn, res);

        gdb.i
            .inner
            .finish_fork(&mut res, target, cur_tid, new_tid)?;

        Ok(StopReasonReporter {
            target,
            res: res.into_state(),
            gdb,
        })
    }

    /// A thread created a new process via vfork.
    ///
    /// This indicates that a vfork system call was executed, creating a new
    /// child process.
    ///
    /// Similar to Fork, but the parent process is suspended until the child
    /// calls exec or exits, as the parent and child temporarily share the
    /// same address space.
    ///
    /// Requires: [`Target::use_vfork_stop_reason`].
    pub fn vfork(
        self,
        cur_tid: T::Tid,
        new_tid: T::Tid,
    ) -> Result<StopReasonReporter<'a, 't, T, C, true, true>, GdbStubError<T::Error, C::Error>>
    {
        let Self {
            target,
            res,
            mut gdb,
        } = self;

        let mut res = ResponseWriter::from_state(&mut gdb.i.conn, res);

        gdb.i
            .inner
            .finish_vfork(&mut res, target, cur_tid, new_tid)?;

        Ok(StopReasonReporter {
            target,
            res: res.into_state(),
            gdb,
        })
    }

    /// A vfork child process has completed its operation.
    ///
    /// This indicates that a child process created by vfork has either called
    /// exec or terminated, so the address spaces of parent and child are no
    /// longer shared.
    ///
    /// Requires: [`Target::use_vforkdone_stop_reason`].
    pub fn vfork_done(
        self,
        tid: T::Tid,
    ) -> Result<StopReasonReporter<'a, 't, T, C, true, true>, GdbStubError<T::Error, C::Error>>
    {
        let Self {
            target,
            res,
            mut gdb,
        } = self;

        let mut res = ResponseWriter::from_state(&mut gdb.i.conn, res);

        gdb.i.inner.finish_vforkdone(&mut res, target, tid)?;

        Ok(StopReasonReporter {
            target,
            res: res.into_state(),
            gdb,
        })
    }

    /// Report that `execve` was called, where `path` is the absolute pathname
    /// of the file that was executed.
    ///
    /// Requires: [`Target::use_exec_stop_reason`].
    pub fn exec(
        self,
        path: impl AsRef<[u8]>,
    ) -> Result<StopReasonReporter<'a, 't, T, C, true, true>, GdbStubError<T::Error, C::Error>>
    {
        let Self {
            target,
            res,
            mut gdb,
        } = self;

        let mut res = ResponseWriter::from_state(&mut gdb.i.conn, res);

        gdb.i.inner.finish_exec(&mut res, target, path.as_ref())?;

        Ok(StopReasonReporter {
            target,
            res: res.into_state(),
            gdb,
        })
    }
}

/// Methods which can only be called from the
/// [`GdbStubStateMachine::Running`] state.
impl<'a, T, C> GdbStubStateMachineInner<'a, state::Running, T, C>
where
    T: Target,
    C: Connection,
{
    /// Report a target stop reason back to GDB.
    pub fn report_stop<'t>(
        mut self,
        target: &'t mut T,
    ) -> StopReasonReporter<'a, 't, T, C, false, false> {
        StopReasonReporter {
            res: ResponseWriter::new(self.borrow_conn(), target.use_rle()).into_state(),
            target,
            gdb: self,
        }
    }

    /// Pass a byte to the GDB stub.
    pub fn incoming_data(
        mut self,
        target: &mut T,
        byte: u8,
    ) -> Result<GdbStubStateMachine<'a, T, C>, GdbStubError<T::Error, C::Error>> {
        let packet_buffer = match self.i.recv_packet.pump(&mut self.i.packet_buffer, byte)? {
            Some(buf) => buf,
            None => return Ok(self.into()),
        };

        let packet = Packet::from_buf(target, packet_buffer).map_err(InternalError::PacketParse)?;
        let state = self
            .i
            .inner
            .handle_packet(target, &mut self.i.conn, packet)?;
        Ok(match state {
            State::Pump => self.transition(state::Running {}).into(),
            State::Disconnect(reason) => self.transition(state::Disconnected { reason }).into(),
            State::DoResume => self.transition(state::Running {}).into(),
            State::CtrlCInterrupt => self
                .transition(state::CtrlCInterrupt { from_idle: false })
                .into(),
        })
    }
}

/// Methods which can only be called from the
/// [`GdbStubStateMachine::CtrlCInterrupt`] state.
impl<'a, T, C> GdbStubStateMachineInner<'a, state::CtrlCInterrupt, T, C>
where
    T: Target,
    C: Connection,
{
    /// Acknowledge the Ctrl-C interrupt.
    ///
    /// Stubs are not required to recognize this interrupt mechanism, and the
    /// precise meaning associated with receipt of the interrupt is
    /// implementation defined. It is perfectly valid to invoke
    /// `interrupt_handled` without actually doing anything (though, given the
    /// utility of supporting Ctrl-C interrupts - this is not advised).
    ///
    /// If you wish to support Ctrl-C interrupts, prior to calling
    /// `interrupt_handled`, you should arrange for `target` to be stopped
    /// (at it's earliest convenience).
    ///
    /// The specifics of "arranging for the `target` to be stopped" will vary
    /// between target implementations.
    ///
    /// Some notes on handling Ctrl-C interrupts:
    ///
    /// - If the target supports debugging of multiple threads and/or processes,
    ///   it should attempt to interrupt _all_ currently-executing threads and
    ///   processes.
    /// - Targets that run "inline" with the `GdbStubStateMachine` (and are
    ///   therefore "implicitly paused" when this event occurs) can set a simple
    ///   boolean flag that a ctrl-c interrupt has occurred, and upon
    ///   looping-around into the `Running` state - use that flag to skip
    ///   resuming the target (and of course - retuning an appropriate stop
    ///   reason).
    /// - If you're unsure which stop reason to report in response to a ctrl-c
    ///   interrupt, reporting a [`Signal::SIGINT`] may be a sensible default.
    pub fn interrupt_handled(self) -> GdbStubStateMachine<'a, T, C> {
        if self.state.from_idle {
            self.transition(state::Idle {}).into()
        } else {
            self.transition(state::Running {}).into()
        }
    }
}

/// Methods which can only be called from the
/// [`GdbStubStateMachine::Disconnected`] state.
impl<'a, T, C> GdbStubStateMachineInner<'a, state::Disconnected, T, C>
where
    T: Target,
    C: Connection,
{
    /// Inspect why the GDB client disconnected.
    pub fn get_reason(&self) -> DisconnectReason {
        self.state.reason
    }

    /// Reuse the existing state machine instance, reentering the idle loop.
    pub fn return_to_idle(self) -> GdbStubStateMachine<'a, T, C> {
        self.transition(state::Idle {}).into()
    }
}
