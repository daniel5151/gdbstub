//! The core [`GdbStub`] type, used to drive a GDB debugging session for a
//! particular [`Target`] over a given [`Connection`].

use core::marker::PhantomData;

use managed::ManagedSlice;

use crate::arch::Arch;
use crate::common::{Signal, Tid};
use crate::conn::{Connection, ConnectionExt};
use crate::protocol::commands::Command;
use crate::protocol::{Packet, ResponseWriter, SpecificIdKind};
use crate::target::ext::base::multithread::ThreadStopReason;
use crate::target::Target;
use crate::SINGLE_THREAD_TID;

mod builder;
mod error;
mod ext;
mod target_result_ext;

pub use builder::{GdbStubBuilder, GdbStubBuilderError};
pub use error::GdbStubError;

use state_machine::GdbStubStateMachine;

use GdbStubError as Error;

/// Describes why the GDB session ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DisconnectReason {
    /// Target exited with given status code
    TargetExited(u8),
    /// Target terminated with given signal
    TargetTerminated(Signal),
    /// GDB issued a disconnect command
    Disconnect,
    /// GDB issued a kill command
    Kill,
}

/// Types and traits related to the [`GdbStub::run_blocking`] interface.
pub mod run_blocking {
    use super::*;

    use crate::conn::ConnectionExt;

    /// A set of user-provided methods required to run a GDB debugging session
    /// using the [`GdbStub::run_blocking`] method.
    ///
    /// Reminder: to use `gdbstub` in a non-blocking manner (e.g: via
    /// async/await, unix polling, from an interrupt handler, etc...) you will
    /// need to interface with the
    /// [`GdbStubStateMachine`](super::GdbStubStateMachine) API directly.
    pub trait BlockingEventLoop {
        /// The Target being driven.
        type Target: Target;
        /// Connection being used to drive the target.
        type Connection: ConnectionExt;

        /// Invoked immediately after the target's `resume` method has been
        /// called. The implementation should block until either the target
        /// reports a stop reason, or if new data was sent over the connection.
        ///
        /// The specific mechanism to "select" between these two events is
        /// implementation specific. Some examples might include: `epoll`,
        /// `select!` across multiple event channels, periodic polling, etc...
        ///
        /// # Single threaded targets
        ///
        /// While the function signature requires returning a
        /// `ThreadStopReason`, single threaded targets should return a
        /// [`StopReason`](crate::target::ext::base::singlethread::StopReason)
        /// instead, using `.into()` to convert it into a `ThreadStopReason`
        /// with the correct "dummy" TID.
        ///
        /// In the future, this API might be changed to avoid exposing this
        /// internal implementation detail.
        fn wait_for_stop_reason(
            target: &mut Self::Target,
            conn: &mut Self::Connection,
        ) -> Result<
            Event<<<Self::Target as Target>::Arch as Arch>::Usize>,
            WaitForStopReasonError<
                <Self::Target as Target>::Error,
                <Self::Connection as Connection>::Error,
            >,
        >;

        /// Invoked when the GDB client sends a Ctrl-C interrupt. The
        /// implementation should handle the interrupt request + return an
        /// appropriate stop reason to report back to the GDB client, or return
        /// `None` if the interrupt should be ignored.
        ///
        /// _Suggestion_: If you're unsure which stop reason to report,
        /// [`ThreadStopReason::Signal(Signal::SIGINT)`](ThreadStopReason) is a
        /// sensible default.
        ///
        /// # Single threaded targets
        ///
        /// While the function signature requires returning a
        /// `ThreadStopReason`, single threaded targets should return a
        /// [`StopReason`](crate::target::ext::base::singlethread::StopReason)
        /// instead, using `.into()` to convert it into a `ThreadStopReason`
        /// with the correct "dummy" TID.
        ///
        /// In the future, this API might be changed to avoid exposing this
        /// internal implementation detail.
        fn on_interrupt(
            target: &mut Self::Target,
        ) -> Result<
            Option<ThreadStopReason<<<Self::Target as Target>::Arch as Arch>::Usize>>,
            <Self::Target as Target>::Error,
        >;
    }

    /// Returned by the `wait_for_stop_reason` closure in
    /// [`GdbStub::run_blocking`]
    pub enum Event<U> {
        /// GDB Client sent data while the target was running.
        IncomingData(u8),
        /// The target has stopped.
        TargetStopped(ThreadStopReason<U>),
    }

    /// Error value returned by the `wait_for_stop_reason` closure in
    /// [`GdbStub::run_blocking`]
    pub enum WaitForStopReasonError<T, C> {
        /// A fatal target error has occurred.
        Target(T),
        /// A fatal connection error has occurred.
        Connection(C),
    }
}

/// Debug a [`Target`] using the GDB Remote Serial Protocol over a given
/// [`Connection`].
pub struct GdbStub<'a, T: Target, C: Connection> {
    conn: C,
    packet_buffer: ManagedSlice<'a, u8>,
    inner: GdbStubImpl<T, C>,
}

impl<'a, T: Target, C: Connection> GdbStub<'a, T, C> {
    /// Create a [`GdbStubBuilder`] using the provided Connection.
    pub fn builder(conn: C) -> GdbStubBuilder<'a, T, C> {
        GdbStubBuilder::new(conn)
    }

    /// Create a new `GdbStub` using the provided connection.
    ///
    /// _Note:_ `new` is only available when the `alloc` feature is enabled, as
    /// it will use a dynamically allocated `Vec` as a packet buffer.
    ///
    /// For fine-grained control over various `GdbStub` options, including the
    /// ability to specify a fixed-size buffer, use the [`GdbStub::builder`]
    /// method instead.
    #[cfg(feature = "alloc")]
    pub fn new(conn: C) -> GdbStub<'a, T, C> {
        GdbStubBuilder::new(conn).build().unwrap()
    }

    /// (Quickstart) Start a GDB remote debugging session using a blocking event
    /// loop.
    ///
    /// This method provides a quick and easy way to get up and running with
    /// `gdbstub` without directly having to immediately interface with the
    /// lower-level [state-machine](`state_machine::GdbStubStateMachine`)
    /// based interface.
    ///
    /// Instead, an implementation simply needs to provide a implementation of
    /// [`run_blocking::BlockingEventLoop`], which is a simplified set
    /// of methods describing how to drive the target.
    ///
    /// `GdbStub::run_blocking` returns once the GDB client closes the debugging
    /// session, or if the target triggers a disconnect.
    ///
    /// Note that this implementation is **blocking**, which many not be
    /// preferred (or suitable) in all cases. To use `gdbstub` in a non-blocking
    /// manner (e.g: via async/await, unix polling, from an interrupt handler,
    /// etc...) you will need to interface with the underlying
    /// [`GdbStubStateMachine`] API directly.
    pub fn run_blocking<E>(
        self,
        target: &mut T,
    ) -> Result<DisconnectReason, Error<T::Error, C::Error>>
    where
        C: ConnectionExt,
        E: run_blocking::BlockingEventLoop<Target = T, Connection = C>,
    {
        let mut gdb = self.run_state_machine(target)?;
        loop {
            gdb = match gdb {
                GdbStubStateMachine::Idle(mut gdb) => {
                    // needs more data, so perform a blocking read on the connection
                    let byte = gdb.borrow_conn().read().map_err(Error::ConnectionRead)?;
                    gdb.incoming_data(target, byte)?
                }

                GdbStubStateMachine::Disconnected(gdb) => {
                    // run_blocking keeps things simple, and doesn't expose a way to re-use the
                    // state machine
                    break Ok(gdb.get_reason());
                }

                GdbStubStateMachine::CtrlCInterrupt(gdb) => {
                    // defer to the implementation on how it wants to handle the interrupt
                    let stop_reason = E::on_interrupt(target).map_err(Error::TargetError)?;
                    gdb.interrupt_handled(target, stop_reason)?
                }

                GdbStubStateMachine::Running(mut gdb) => {
                    use run_blocking::{Event as BlockingEventLoopEvent, WaitForStopReasonError};

                    // block waiting for the target to return a stop reason
                    let event = E::wait_for_stop_reason(target, gdb.borrow_conn());
                    match event {
                        Ok(BlockingEventLoopEvent::TargetStopped(stop_reason)) => {
                            gdb.report_stop(target, stop_reason)?
                        }

                        Ok(BlockingEventLoopEvent::IncomingData(byte)) => {
                            gdb.incoming_data(target, byte)?
                        }

                        Err(WaitForStopReasonError::Target(e)) => {
                            break Err(Error::TargetError(e));
                        }
                        Err(WaitForStopReasonError::Connection(e)) => {
                            break Err(Error::ConnectionRead(e));
                        }
                    }
                }
            }
        }
    }

    /// Starts a GDB remote debugging session, converting this instance of
    /// `GdbStub` into a
    /// [`GdbStubStateMachine`](state_machine::GdbStubStateMachine) that is
    /// ready to receive data.
    pub fn run_state_machine(
        mut self,
        target: &mut T,
    ) -> Result<state_machine::GdbStubStateMachine<'a, T, C>, Error<T::Error, C::Error>> {
        // Check if the target hasn't explicitly opted into implicit sw breakpoints
        {
            let support_software_breakpoints = target
                .support_breakpoints()
                .map(|ops| ops.support_sw_breakpoint().is_some())
                .unwrap_or(false);

            if !support_software_breakpoints && !target.use_implicit_sw_breakpoints() {
                return Err(Error::ImplicitSwBreakpoints);
            }
        }

        // Check if the target supports single stepping as an optional feature
        {
            use crate::target::ext::base::ResumeOps;

            if let Some(ops) = target.base_ops().resume_ops() {
                let support_single_step = match ops {
                    ResumeOps::SingleThread(ops) => ops.support_single_step().is_some(),
                    ResumeOps::MultiThread(ops) => ops.support_single_step().is_some(),
                };

                if !support_single_step && !target.use_optional_single_step() {
                    return Err(Error::UnconditionalSingleStep);
                }
            }
        }

        // Perform any connection initialization
        {
            self.conn
                .on_session_start()
                .map_err(Error::ConnectionInit)?;
        }

        Ok(state_machine::GdbStubStateMachineInner::from_plain_gdbstub(self).into())
    }
}

/// Low-level state-machine interface that underpins [`GdbStub`].
///
/// TODO: write some proper documentation + examples of how to interface with
/// this API.
///
/// # Hey, what gives? Where are all the docs!?
///
/// Sorry about that!
///
/// `gdbstub` 0.6 turned out ot be a pretty massive release, and documenting
/// everything has proven to be a gargantuan task.
///
/// There are quite a few folks asking that I publish 0.6 to crates.io, so to
/// avoid blocking the release any further, I've decided to leave this bit of
/// the API sparsely documented...
///
/// If you're interested in using this API directly (e.g: to integrate `gdbstub`
/// into a `no_std` project, or to use `gdbstub` in a non-blocking manner
/// alongside `async/await` / a project specific event loop), your best bet
/// would be to review the following bits of code to get a feel for the API:
///
/// - The implementation of [`GdbStub::run_blocking`]
/// - Implementations of [`BlockingEventLoop`](run_blocking::BlockingEventLoop)
///   used alongside `GdbStub::run_blocking` (e.g: the in-tree `armv4t` /
///   `armv4t_multicore` examples)
/// - Real-world projects using the API
///     - The best example of this (at the time of writing) is the code at [`vmware-labs/node-replicated-kernel`](https://github.com/vmware-labs/node-replicated-kernel/blob/4326704aaf3c0052e614dcde2a788a8483224394/kernel/src/arch/x86_64/gdb/mod.rs#L106)
///
/// If you have any questions, feel free to open a discussion thread over at the
/// `gdbstub` [GitHub repo](https://github.com/daniel5151/gdbstub/discussions)
pub mod state_machine {
    use super::*;

    use crate::protocol::recv_packet::RecvPacketStateMachine;

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
        Idle(GdbStubStateMachineInner<'a, state::Idle<T>, T, C>),
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
        pub struct Idle<T: Target> {
            pub(crate) deferred_ctrlc_stop_reason:
                Option<ThreadStopReason<<<T as Target>::Arch as Arch>::Usize>>,
        }

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

    impl_from_inner!(Idle<T>);
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
    impl<'a, S, T: Target, C: Connection> GdbStubStateMachineInner<'a, S, T, C> {
        /// Return a mutable reference to the underlying connection.
        pub fn borrow_conn(&mut self) -> &mut C {
            &mut self.i.conn
        }
    }

    /// Methods which can only be called from the [`GdbStubStateMachine::Idle`]
    /// state.
    impl<'a, T: Target, C: Connection> GdbStubStateMachineInner<'a, state::Idle<T>, T, C> {
        /// Internal entrypoint into the state machine.
        pub(crate) fn from_plain_gdbstub(
            stub: GdbStub<'a, T, C>,
        ) -> GdbStubStateMachineInner<'a, state::Idle<T>, T, C> {
            GdbStubStateMachineInner {
                i: GdbStubStateMachineReallyInner {
                    conn: stub.conn,
                    packet_buffer: stub.packet_buffer,
                    recv_packet: RecvPacketStateMachine::new(),
                    inner: stub.inner,
                },
                state: state::Idle {
                    deferred_ctrlc_stop_reason: None,
                },
            }
        }

        /// Pass a byte to the GDB stub.
        pub fn incoming_data(
            mut self,
            target: &mut T,
            byte: u8,
        ) -> Result<GdbStubStateMachine<'a, T, C>, Error<T::Error, C::Error>> {
            let packet_buffer = match self.i.recv_packet.pump(&mut self.i.packet_buffer, byte)? {
                Some(buf) => buf,
                None => return Ok(self.into()),
            };

            let packet = Packet::from_buf(target, packet_buffer).map_err(Error::PacketParse)?;
            let state = self
                .i
                .inner
                .handle_packet(target, &mut self.i.conn, packet)?;
            Ok(match state {
                State::Pump => self.into(),
                State::Disconnect(reason) => self.transition(state::Disconnected { reason }).into(),
                State::DeferredStopReason => {
                    match self.state.deferred_ctrlc_stop_reason {
                        // if we were interrupted while idle, immediately report the deferred stop
                        // reason after transitioning into the running state
                        Some(reason) => {
                            return self
                                .transition(state::Running {})
                                .report_stop(target, reason)
                        }
                        // otherwise, just transition into the running state as usual
                        None => self.transition(state::Running {}).into(),
                    }
                }
                State::CtrlCInterrupt => self
                    .transition(state::CtrlCInterrupt { from_idle: true })
                    .into(),
            })
        }
    }

    /// Methods which can only be called from the
    /// [`GdbStubStateMachine::Running`] state.
    impl<'a, T: Target, C: Connection> GdbStubStateMachineInner<'a, state::Running, T, C> {
        /// Report a target stop reason back to GDB.
        ///
        /// # Single threaded targets
        ///
        /// While the function signature requires returning a
        /// [`ThreadStopReason`], single threaded targets should return a
        /// [`StopReason`](crate::target::ext::base::singlethread::StopReason)
        /// instead, using `.into()` to convert it into a `ThreadStopReason`
        /// with the correct "dummy" TID.
        ///
        /// In the future, this API might be changed to avoid exposing this
        /// internal implementation detail.
        pub fn report_stop(
            mut self,
            target: &mut T,
            reason: ThreadStopReason<<T::Arch as Arch>::Usize>,
        ) -> Result<GdbStubStateMachine<'a, T, C>, Error<T::Error, C::Error>> {
            let mut res = ResponseWriter::new(&mut self.i.conn);
            let event = self.i.inner.finish_exec(&mut res, target, reason)?;
            res.flush()?;

            Ok(match event {
                ext::FinishExecStatus::Handled => self
                    .transition(state::Idle {
                        deferred_ctrlc_stop_reason: None,
                    })
                    .into(),
                ext::FinishExecStatus::Disconnect(reason) => {
                    self.transition(state::Disconnected { reason }).into()
                }
            })
        }

        /// Pass a byte to the GDB stub.
        ///
        /// NOTE: unlike the `incoming_data` method in the `state::Idle` state,
        /// this method does not perform any state transitions, and will
        /// return a `GdbStubStateMachineInner` in the `state::Running` state.
        pub fn incoming_data(
            mut self,
            target: &mut T,
            byte: u8,
        ) -> Result<GdbStubStateMachine<'a, T, C>, Error<T::Error, C::Error>> {
            let packet_buffer = match self.i.recv_packet.pump(&mut self.i.packet_buffer, byte)? {
                Some(buf) => buf,
                None => return Ok(self.into()),
            };

            let packet = Packet::from_buf(target, packet_buffer).map_err(Error::PacketParse)?;
            let state = self
                .i
                .inner
                .handle_packet(target, &mut self.i.conn, packet)?;
            Ok(match state {
                State::Pump => self.transition(state::Running {}).into(),
                State::Disconnect(reason) => self.transition(state::Disconnected { reason }).into(),
                State::DeferredStopReason => self.transition(state::Running {}).into(),
                State::CtrlCInterrupt => self
                    .transition(state::CtrlCInterrupt { from_idle: false })
                    .into(),
            })
        }
    }

    /// Methods which can only be called from the
    /// [`GdbStubStateMachine::CtrlCInterrupt`] state.
    impl<'a, T: Target, C: Connection> GdbStubStateMachineInner<'a, state::CtrlCInterrupt, T, C> {
        /// The target has acknowledged the clients Ctrl-C interrupt, and taken
        /// any appropriate actions to fulfil the interrupt request.
        ///
        /// Some notes on handling Ctrl-C interrupts:
        ///
        /// - Stubs are not required to recognize these interrupt mechanisms,
        ///   and the precise meaning associated with receipt of the interrupt
        ///   is implementation defined.
        ///   - Passing `None` as the `stop_reason` will ignore the Ctrl-C
        ///     interrupt, and return the state machine to whatever state it was
        ///     in before being interrupted.
        /// - If the target supports debugging of multiple threads and/or
        ///   processes, it should attempt to interrupt all currently-executing
        ///   threads and processes.
        /// - If the stub is successful at interrupting the running program, it
        ///   should send one of the stop reply packets (see Stop Reply Packets)
        ///   to GDB as a result of successfully stopping the program
        pub fn interrupt_handled(
            self,
            target: &mut T,
            stop_reason: Option<ThreadStopReason<<T::Arch as Arch>::Usize>>,
        ) -> Result<GdbStubStateMachine<'a, T, C>, Error<T::Error, C::Error>> {
            if self.state.from_idle {
                // target is stopped - we cannot report the stop reason yet
                Ok(self
                    .transition(state::Idle {
                        deferred_ctrlc_stop_reason: stop_reason,
                    })
                    .into())
            } else {
                // target is running - we can immediately report the stop reason
                match stop_reason {
                    Some(reason) => self
                        .transition(state::Running {})
                        .report_stop(target, reason),
                    None => Ok(self
                        .transition(state::Idle {
                            deferred_ctrlc_stop_reason: None,
                        })
                        .into()),
                }
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
            self.transition(state::Idle {
                deferred_ctrlc_stop_reason: None,
            })
            .into()
        }
    }
}

enum State {
    Pump,
    DeferredStopReason,
    CtrlCInterrupt,
    Disconnect(DisconnectReason),
}

struct GdbStubImpl<T: Target, C: Connection> {
    _target: PhantomData<T>,
    _connection: PhantomData<C>,

    current_mem_tid: Tid,
    current_resume_tid: SpecificIdKind,
    no_ack_mode: bool,
}

enum HandlerStatus {
    Handled,
    NeedsOk,
    DeferredStopReason,
    Disconnect(DisconnectReason),
}

impl<T: Target, C: Connection> GdbStubImpl<T, C> {
    fn new() -> GdbStubImpl<T, C> {
        GdbStubImpl {
            _target: PhantomData,
            _connection: PhantomData,

            // NOTE: `current_mem_tid` and `current_resume_tid` are never queried prior to being set
            // by the GDB client (via the 'H' packet), so it's fine to use dummy values here.
            //
            // The alternative would be to use `Option`, and while this would be more "correct", it
            // would introduce a _lot_ of noisy and heavy error handling logic all over the place.
            //
            // Plus, even if the GDB client is acting strangely and doesn't overwrite these values,
            // the target will simply return a non-fatal error, which is totally fine.
            current_mem_tid: SINGLE_THREAD_TID,
            current_resume_tid: SpecificIdKind::WithId(SINGLE_THREAD_TID),
            no_ack_mode: false,
        }
    }

    fn handle_packet(
        &mut self,
        target: &mut T,
        conn: &mut C,
        packet: Packet<'_>,
    ) -> Result<State, Error<T::Error, C::Error>> {
        match packet {
            Packet::Ack => Ok(State::Pump),
            Packet::Nack => Err(Error::ClientSentNack),
            Packet::Interrupt => {
                debug!("<-- interrupt packet");
                Ok(State::CtrlCInterrupt)
            }
            Packet::Command(command) => {
                // Acknowledge the command
                if !self.no_ack_mode {
                    conn.write(b'+').map_err(Error::ConnectionWrite)?;
                }

                let mut res = ResponseWriter::new(conn);
                let disconnect_reason = match self.handle_command(&mut res, target, command) {
                    Ok(HandlerStatus::Handled) => None,
                    Ok(HandlerStatus::NeedsOk) => {
                        res.write_str("OK")?;
                        None
                    }
                    Ok(HandlerStatus::DeferredStopReason) => return Ok(State::DeferredStopReason),
                    Ok(HandlerStatus::Disconnect(reason)) => Some(reason),
                    // HACK: handling this "dummy" error is required as part of the
                    // `TargetResultExt::handle_error()` machinery.
                    Err(Error::NonFatalError(code)) => {
                        res.write_str("E")?;
                        res.write_num(code)?;
                        None
                    }
                    Err(Error::TargetError(e)) => {
                        // unlike all other errors which are "unrecoverable" in the sense that
                        // the GDB session cannot continue, there's still a chance that a target
                        // might want to keep the debugging session alive to do a "post-mortem"
                        // analysis. As such, we simply report a standard TRAP stop reason.
                        let mut res = ResponseWriter::new(conn);
                        res.write_str("S05")?;
                        res.flush()?;
                        return Err(Error::TargetError(e));
                    }
                    Err(e) => return Err(e),
                };

                // every response needs to be flushed, _except_ for the response to a kill
                // packet, but ONLY when extended mode is NOT implemented.
                let is_kill = matches!(disconnect_reason, Some(DisconnectReason::Kill));
                if !(target.support_extended_mode().is_none() && is_kill) {
                    res.flush()?;
                }

                let state = match disconnect_reason {
                    Some(reason) => State::Disconnect(reason),
                    None => State::Pump,
                };

                Ok(state)
            }
        }
    }

    fn handle_command(
        &mut self,
        res: &mut ResponseWriter<C>,
        target: &mut T,
        cmd: Command<'_>,
    ) -> Result<HandlerStatus, Error<T::Error, C::Error>> {
        match cmd {
            // `handle_X` methods are defined in the `ext` module
            Command::Base(cmd) => self.handle_base(res, target, cmd),
            Command::Resume(cmd) => self.handle_stop_resume(res, target, cmd),
            Command::XUpcasePacket(cmd) => self.handle_x_upcase_packet(res, target, cmd),
            Command::SingleRegisterAccess(cmd) => {
                self.handle_single_register_access(res, target, cmd)
            }
            Command::Breakpoints(cmd) => self.handle_breakpoints(res, target, cmd),
            Command::CatchSyscalls(cmd) => self.handle_catch_syscalls(res, target, cmd),
            Command::ExtendedMode(cmd) => self.handle_extended_mode(res, target, cmd),
            Command::MonitorCmd(cmd) => self.handle_monitor_cmd(res, target, cmd),
            Command::SectionOffsets(cmd) => self.handle_section_offsets(res, target, cmd),
            Command::ReverseCont(cmd) => self.handle_reverse_cont(res, target, cmd),
            Command::ReverseStep(cmd) => self.handle_reverse_step(res, target, cmd),
            Command::MemoryMap(cmd) => self.handle_memory_map(res, target, cmd),
            Command::HostIo(cmd) => self.handle_host_io(res, target, cmd),
            Command::ExecFile(cmd) => self.handle_exec_file(res, target, cmd),
            Command::Auxv(cmd) => self.handle_auxv(res, target, cmd),
            // in the worst case, the command could not be parsed...
            Command::Unknown(cmd) => {
                // HACK: if the user accidentally sends a resume command to a
                // target without resume support, inform them of their mistake +
                // return a dummy stop reason.
                if target.base_ops().resume_ops().is_none() && target.use_resume_stub() {
                    let is_resume_pkt = cmd
                        .get(0)
                        .map(|c| matches!(c, b'c' | b'C' | b's' | b'S'))
                        .unwrap_or(false);

                    if is_resume_pkt {
                        warn!("attempted to resume target without resume support!");

                        // TODO: omit this message if non-stop mode is active
                        {
                            let mut res = ResponseWriter::new(res.as_conn());
                            res.write_str("O")?;
                            res.write_hex_buf(b"target has not implemented `support_resume()`\n")?;
                            res.flush()?;
                        }

                        res.write_str("S05")?;
                    }
                }

                info!("Unknown command: {:?}", core::str::from_utf8(cmd));
                Ok(HandlerStatus::Handled)
            }
        }
    }
}
