//! The core [`GdbStub`] type, used to drive a GDB debugging session for a
//! particular [`Target`] over a given [`Connection`].

pub use builder::GdbStubBuilder;
pub use builder::GdbStubBuilderError;
pub use core_impl::DisconnectReason;
pub use error::GdbStubError;
use stop_reason::BaseStopReason;

mod builder;
mod core_impl;
mod error;
mod stop_reason;

pub mod state_machine;

use self::error::InternalError;
use crate::conn::Connection;
use crate::conn::ConnectionExt;
use crate::target::Target;
use managed::ManagedSlice;

/// Types and traits related to the [`GdbStub::run_blocking`] interface.
// DEVNOTE: There is nothing in this module that makes it _required_ to exist in
// `gdbstub` core. Indeed, it could just as well be hoisted into an entirely
// separate crate (a-la `gdbstub_arch`) and distributed separately. Not that we
// should do that, of course. Having a "blessed" quick-start path is great for
// lowering the barrier to entry!
pub mod run_blocking {
    use super::*;
    use crate::conn::ConnectionExt;
    use crate::stub::state_machine::ReportStop;
    use crate::stub::state_machine::StopReason;
    use crate::IsValidTid;

    /// Simple interface to a running [`GdbStubStateMachine`], used in
    /// [`BlockingEventLoop::wait_for_stop_reason`].
    ///
    /// [`GdbStubStateMachine`]: state_machine::GdbStubStateMachine
    pub struct SimpleStub<'a, T: Target, C: Connection, Tid: IsValidTid>(
        pub(crate) state_machine::GdbStubStateMachineInner<'a, state_machine::state::Running, T, C>,
        pub(crate) std::marker::PhantomData<Tid>,
    );

    /// Opaque type representing an event that was occurred in
    /// [`BlockingEventLoop::wait_for_stop_reason`].
    ///
    /// Created via [`SimpleStub`].
    pub struct Event<'a, T: Target, C: Connection>(
        pub(crate)  Result<
            state_machine::GdbStubStateMachine<'a, T, C>,
            GdbStubError<<T as Target>::Error, <C as Connection>::Error>,
        >,
    );

    impl<'a, T: Target, C: Connection, Tid: IsValidTid> SimpleStub<'a, T, C, Tid> {
        /// Return a mutable reference to the underlying connection.
        pub fn borrow_conn(&mut self) -> &mut C {
            self.0.borrow_conn()
        }

        /// Report a target stop reason back to GDB.
        pub fn report_stop(
            self,
            target: &mut T,
            report: impl FnOnce(ReportStop<T, Tid>) -> StopReason<T>,
        ) -> Event<'a, T, C> {
            Event(self.0.report_stop(target, report))
        }

        /// Pass a byte to the GDB stub.
        pub fn incoming_data(self, target: &mut T, byte: u8) -> Event<'a, T, C> {
            Event(self.0.incoming_data(target, byte))
        }
    }

    /// A set of user-provided methods required to run a GDB debugging session
    /// using the [`GdbStub::run_blocking`] method.
    ///
    /// Reminder: to use `gdbstub` in a non-blocking manner (e.g: via
    /// async/await, unix polling, from an interrupt handler, etc...) you will
    /// need to interface with the
    /// [`GdbStubStateMachine`](state_machine::GdbStubStateMachine) API
    /// directly.
    pub trait BlockingEventLoop {
        /// The Target being driven.
        type Target: Target;
        /// Connection being used to drive the target.
        type Connection: ConnectionExt;
        /// What kind of target this is (singlethread vs. multithread)
        type Tid: IsValidTid;

        /// Invoked after the target's `resume` method has been called.
        ///
        /// Implementations resume the target, and run until either:
        /// - the target has stopped
        /// - new data has arrived over the connection
        ///
        /// The specific mechanism to concurrently "select" between these two
        /// events is implementation specific. Some examples might
        /// include: `epoll`, `select!` across multiple event channels,
        /// periodic polling, etc...
        ///
        /// Events are reported via methods on the provided [`SimpleStub`].
        fn wait_for_stop_reason<'a>(
            target: &mut Self::Target,
            simple_stub: SimpleStub<'a, Self::Target, Self::Connection, Self::Tid>,
        ) -> Result<
            Event<'a, Self::Target, Self::Connection>,
            WaitForStopReasonError<
                <Self::Target as Target>::Error,
                <Self::Connection as Connection>::Error,
            >,
        >;

        /// Invoked when the GDB client sends a Ctrl-C interrupt.
        ///
        /// Stubs are not required to recognize this interrupt mechanism, and
        /// the precise meaning associated with receipt of the interrupt is
        /// implementation defined, so leaving this method as the default no-op
        /// is reasonable (though, given the utility of supporting Ctrl-C
        /// interrupts - this is not advised).
        ///
        /// To support interrupts, override this method with logic to notify
        /// the `target` of the interrupt request, and arrange for it to be
        /// stopped (at it's earliest convenience).
        ///
        /// The specifics of "arranging for it to be stopped" will vary between
        /// targets. For example:
        ///
        /// 1. In targets that runs "inline" with the `BlockingEventLoop`, it
        ///    may be sufficient to set a simple boolean flag on the `target`
        ///    that can be queried in `wait_for_stop_reason()` prior to resuming
        ///    execution (i.e: if set, immediately report a stop reason).
        /// 2. In targets that run asynchronously from the `BlockingEventLoop`
        ///    (such as those running on separate threads), `on_interrupt()`
        ///    might send a "message" to the target to "inject" the interrupt
        ///    (e.g: over a chanel, via a shared atomic, etc...), and return
        ///    from the method. At this point, the target can then process the
        ///    interrupt at its leisure, reporting the event via an
        ///    `Event::StopReason` in `wait_for_stop_reason()` as usual.
        ///
        /// _Suggestion_: If you're unsure which stop reason to report in
        /// response to a ctrl-c interrupt,
        /// [`BaseStopReason::Signal(Signal::SIGINT)`] may be a sensible
        /// default.
        ///
        /// [`BaseStopReason::Signal(Signal::SIGINT)`]:
        /// crate::stub::BaseStopReason::Signal
        fn on_interrupt(target: &mut Self::Target) -> Result<(), <Self::Target as Target>::Error> {
            let _ = target;
            Ok(())
        }
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
    inner: core_impl::GdbStubImpl<T, C>,
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
    /// lower-level [state-machine](state_machine::GdbStubStateMachine)
    /// based interface.
    ///
    /// Implementations need only to provide a custom
    /// [`run_blocking::BlockingEventLoop`], which is then used by
    /// `run_blocking` in order to drive Target execution.
    ///
    /// `GdbStub::run_blocking` only returns once the GDB client closes the
    /// debugging session, or if the Target triggers a disconnect.
    ///
    /// As the name implies - this helper method is **blocking**, which many not
    /// be preferable (or suitable) in all use-cases. To use `gdbstub` in a
    /// fully non-blocking manner (e.g: via async/await, unix polling, embedded
    /// system interrupt handlers, etc...) you will need to interface with the
    /// underlying
    /// [`GdbStubStateMachine`](state_machine::GdbStubStateMachine) API
    /// directly.
    pub fn run_blocking<E>(
        self,
        target: &mut T,
    ) -> Result<DisconnectReason, GdbStubError<T::Error, C::Error>>
    where
        C: ConnectionExt,
        E: run_blocking::BlockingEventLoop<Target = T, Connection = C>,
    {
        let mut gdb = self.run_state_machine(target)?;
        loop {
            gdb = match gdb {
                state_machine::GdbStubStateMachine::Idle(mut gdb) => {
                    // needs more data, so perform a blocking read on the connection
                    let byte = gdb.borrow_conn().read().map_err(InternalError::conn_read)?;
                    gdb.incoming_data(target, byte)?
                }

                state_machine::GdbStubStateMachine::Disconnected(gdb) => {
                    // run_blocking keeps things simple, and doesn't expose a way to re-use the
                    // state machine
                    break Ok(gdb.get_reason());
                }

                state_machine::GdbStubStateMachine::CtrlCInterrupt(gdb) => {
                    E::on_interrupt(target).map_err(GdbStubError::from_target_error)?;
                    gdb.interrupt_handled()
                }

                state_machine::GdbStubStateMachine::Running(gdb) => {
                    use run_blocking::WaitForStopReasonError;

                    // block waiting for the target to return a stop reason
                    let res = E::wait_for_stop_reason(
                        target,
                        run_blocking::SimpleStub(gdb, std::marker::PhantomData),
                    );
                    match res {
                        Ok(run_blocking::Event(gdb)) => gdb?,
                        Err(WaitForStopReasonError::Target(e)) => {
                            break Err(InternalError::TargetError(e).into());
                        }
                        Err(WaitForStopReasonError::Connection(e)) => {
                            break Err(InternalError::conn_read(e).into());
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
    ) -> Result<state_machine::GdbStubStateMachine<'a, T, C>, GdbStubError<T::Error, C::Error>>
    {
        // Check if the target hasn't explicitly opted into implicit sw breakpoints
        {
            let support_software_breakpoints = target
                .support_breakpoints()
                .map(|ops| ops.support_sw_breakpoint().is_some())
                .unwrap_or(false);

            if !support_software_breakpoints && !target.guard_rail_implicit_sw_breakpoints() {
                return Err(InternalError::ImplicitSwBreakpoints.into());
            }
        }

        // Perform any connection initialization
        {
            self.conn
                .on_session_start()
                .map_err(InternalError::conn_init)?;
        }

        Ok(state_machine::GdbStubStateMachineInner::from_plain_gdbstub(self).into())
    }
}
