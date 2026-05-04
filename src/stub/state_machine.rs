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

use super::core_impl::FinishExecStatus;
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
use crate::stub::error::GdbStubError;
use crate::stub::error::InternalError;
use crate::stub::BaseStopReason;
use crate::target::ext::breakpoints::WatchKind;
use crate::target::Target;
use crate::IsValidTid;
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
impl<S, T: Target, C: Connection> GdbStubStateMachineInner<'_, S, T, C> {
    /// Return a mutable reference to the underlying connection.
    pub fn borrow_conn(&mut self) -> &mut C {
        &mut self.i.conn
    }
}

/// Methods which can only be called from the [`GdbStubStateMachine::Idle`]
/// state.
impl<'a, T: Target, C: Connection> GdbStubStateMachineInner<'a, state::Idle, T, C> {
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

/// Handle to report stop reasons.
pub struct ReportStop<T: Target, Tid> {
    _target: std::marker::PhantomData<T>,
    _tid: std::marker::PhantomData<Tid>,
}

/// Stop reason that can be reported back to GDB.
///
/// Constructed via [`ReportStop`].
pub struct StopReason<T: Target> {
    reason: BaseStopReason<crate::common::Tid, <T::Arch as Arch>::Usize>,
}

impl<T: Target, Tid: IsValidTid> ReportStop<T, Tid> {
    /// Completed the single-step request.
    pub fn done_step(self) -> StopReason<T> {
        StopReason {
            reason: BaseStopReason::DoneStep,
        }
    }

    /// The process terminated with the specified signal number.
    pub fn terminated(self, signal: Signal) -> StopReason<T> {
        StopReason {
            reason: BaseStopReason::Terminated(signal),
        }
    }

    /// The process exited with the specified exit status.
    pub fn exited(self, status: u8) -> StopReason<T> {
        StopReason {
            reason: BaseStopReason::Exited(status),
        }
    }

    /// The program received a signal.
    pub fn signal(self, signal: Signal) -> StopReason<T> {
        StopReason {
            reason: BaseStopReason::Signal(signal),
        }
    }

    /// A specific thread received a signal.
    pub fn signal_with_thread(self, signal: Signal, tid: Tid) -> StopReason<T> {
        StopReason {
            reason: BaseStopReason::SignalWithThread {
                tid: tid.into_fully_qualified_tid(),
                signal,
            },
        }
    }

    /// A thread hit a software breakpoint (e.g. due to a trap instruction).
    ///
    /// Requires: [`SwBreakpoint`].
    ///
    /// NOTE: This does not necessarily have to be a breakpoint configured by
    /// the client/user of the current GDB session.
    ///
    /// [`SwBreakpoint`]: crate::target::ext::breakpoints::SwBreakpoint
    pub fn swbreak(self, tid: Tid) -> StopReason<T>
    where
        T: crate::target::ext::breakpoints::SwBreakpoint,
    {
        StopReason {
            reason: BaseStopReason::SwBreak(tid.into_fully_qualified_tid()),
        }
    }

    /// A thread hit a hardware breakpoint.
    ///
    /// Requires: [`HwBreakpoint`].
    ///
    /// [`HwBreakpoint`]: crate::target::ext::breakpoints::HwBreakpoint
    pub fn hwbreak(self, tid: Tid) -> StopReason<T>
    where
        T: crate::target::ext::breakpoints::HwBreakpoint,
    {
        StopReason {
            reason: BaseStopReason::HwBreak(tid.into_fully_qualified_tid()),
        }
    }

    /// A thread hit a watchpoint.
    ///
    /// Requires: [`HwWatchpoint`].
    ///
    /// [`HwWatchpoint`]: crate::target::ext::breakpoints::HwWatchpoint
    pub fn watch(
        self,
        tid: Tid,
        kind: WatchKind,
        addr: <<T as Target>::Arch as Arch>::Usize,
    ) -> StopReason<T>
    where
        T: crate::target::ext::breakpoints::HwWatchpoint,
    {
        StopReason {
            reason: BaseStopReason::Watch {
                tid: tid.into_fully_qualified_tid(),
                kind,
                addr,
            },
        }
    }
}

/// Methods which can only be called from the
/// [`GdbStubStateMachine::Running`] state.
impl<'a, T: Target, C: Connection> GdbStubStateMachineInner<'a, state::Running, T, C> {
    /// Report a target stop reason back to GDB.
    pub fn report_stop<Tid: IsValidTid>(
        self,
        target: &mut T,
        report: impl FnOnce(ReportStop<T, Tid>) -> StopReason<T>,
    ) -> Result<GdbStubStateMachine<'a, T, C>, GdbStubError<T::Error, C::Error>> {
        self.report_stop_impl(target, report, None)
    }

    /// Report a target stop reason back to GDB, including expedited register
    /// values in the stop reply T-packet.
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
    /// This may be useful to use, rather than [`Self::report_stop`], when we
    /// want to provide register values immediately to, for example, avoid a
    /// round-trip, or work around a quirk/bug in a debugger that does not
    /// otherwise request new register values.
    ///
    /// [`RegId::to_raw_id`]: crate::arch::RegId::to_raw_id
    pub fn report_stop_with_regs<Tid: IsValidTid>(
        self,
        target: &mut T,
        report: impl FnOnce(ReportStop<T, Tid>) -> StopReason<T>,
        // FUTURE: (breaking) explore adding a `RegIdWithVal` construct, in
        // order to tighten up this typing even further?
        regs: &mut dyn Iterator<Item = (<<T as Target>::Arch as Arch>::RegId, &[u8])>,
    ) -> Result<GdbStubStateMachine<'a, T, C>, GdbStubError<T::Error, C::Error>> {
        self.report_stop_impl(target, report, Some(regs))
    }

    /// Shared implementation for the `report_stop`/`report_stop_with_regs` API.
    /// Takes an `Option` around the `&mut dyn Iterator` to avoid making a
    /// dynamic vtable dispatch in the common `report_stop` case.
    fn report_stop_impl<Tid: IsValidTid>(
        mut self,
        target: &mut T,
        report: impl FnOnce(ReportStop<T, Tid>) -> StopReason<T>,
        regs: Option<&mut dyn Iterator<Item = (<<T as Target>::Arch as Arch>::RegId, &[u8])>>,
    ) -> Result<GdbStubStateMachine<'a, T, C>, GdbStubError<T::Error, C::Error>> {
        let reason = report(ReportStop {
            _target: std::marker::PhantomData,
            _tid: std::marker::PhantomData,
        })
        .reason;
        let mut res = ResponseWriter::new(&mut self.i.conn, target.use_rle());
        let event = self.i.inner.finish_exec(&mut res, target, reason)?;

        if let Some(regs) = regs {
            if reason.is_t_packet() {
                for (reg_id, value) in regs {
                    let reg = reg_id.to_raw_id().ok_or(InternalError::MissingToRawId)?;
                    res.write_num(reg).map_err(InternalError::from)?;
                    res.write_str(":").map_err(InternalError::from)?;
                    res.write_hex_buf(value).map_err(InternalError::from)?;
                    res.write_str(";").map_err(InternalError::from)?;
                }
            }
        }

        res.flush().map_err(InternalError::from)?;

        Ok(match event {
            FinishExecStatus::Handled => self.transition(state::Idle {}).into(),
            FinishExecStatus::Disconnect(reason) => {
                self.transition(state::Disconnected { reason }).into()
            }
        })
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
impl<'a, T: Target, C: Connection> GdbStubStateMachineInner<'a, state::CtrlCInterrupt, T, C> {
    /// Acknowledge the Ctrl-C interrupt.
    ///
    /// Stubs are not required to recognize this interrupt mechanism, and
    /// the precise meaning associated with receipt of the interrupt is
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
    ///   interrupt, [`BaseStopReason::Signal(Signal::SIGINT)`] may be a
    ///   sensible default.
    ///
    /// [`BaseStopReason::Signal(Signal::SIGINT)`]:
    /// crate::stub::BaseStopReason::Signal
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
impl<'a, T: Target, C: Connection> GdbStubStateMachineInner<'a, state::Disconnected, T, C> {
    /// Inspect why the GDB client disconnected.
    pub fn get_reason(&self) -> DisconnectReason {
        self.state.reason
    }

    /// Reuse the existing state machine instance, reentering the idle loop.
    pub fn return_to_idle(self) -> GdbStubStateMachine<'a, T, C> {
        self.transition(state::Idle {}).into()
    }
}
