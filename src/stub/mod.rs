//! The core [`GdbStub`] type, used to drive a GDB debugging session for a
//! particular [`Target`] over a given [`Connection`].

use managed::ManagedSlice;

use crate::conn::{Connection, ConnectionExt};
use crate::target::Target;

mod builder;
mod core_impl;
mod error;
mod stop_reason;

pub mod state_machine;

pub use builder::{GdbStubBuilder, GdbStubBuilderError};
pub use core_impl::DisconnectReason;
pub use error::GdbStubError;
pub use stop_reason::{
    BaseStopReason, IntoStopReason, MultiThreadStopReason, SingleThreadStopReason,
};

use GdbStubError as Error;

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
    /// [`GdbStubStateMachine`](state_machine::GdbStubStateMachine) API
    /// directly.
    pub trait BlockingEventLoop {
        /// The Target being driven.
        type Target: Target;
        /// Connection being used to drive the target.
        type Connection: ConnectionExt;

        /// Which variant of the `StopReason` type should be used. Single
        /// threaded targets should use [`SingleThreadStopReason`], whereas
        /// multi threaded targets should use [`MultiThreadStopReason`].
        ///
        /// [`SingleThreadStopReason`]: crate::stub::SingleThreadStopReason
        /// [`MultiThreadStopReason`]: crate::stub::MultiThreadStopReason
        type StopReason: IntoStopReason<Self::Target>;

        /// Invoked immediately after the target's `resume` method has been
        /// called. The implementation should block until either the target
        /// reports a stop reason, or if new data was sent over the connection.
        ///
        /// The specific mechanism to "select" between these two events is
        /// implementation specific. Some examples might include: `epoll`,
        /// `select!` across multiple event channels, periodic polling, etc...
        fn wait_for_stop_reason(
            target: &mut Self::Target,
            conn: &mut Self::Connection,
        ) -> Result<
            Event<Self::StopReason>,
            WaitForStopReasonError<
                <Self::Target as Target>::Error,
                <Self::Connection as Connection>::Error,
            >,
        >;

        /// Invoked when the GDB client sends a Ctrl-C interrupt.
        ///
        /// Depending on how the target is implemented, it may or may not make
        /// sense to immediately return a stop reason as part of handling the
        /// Ctrl-C interrupt. e.g: in some cases, it may be better to send the
        /// target a signal upon receiving a Ctrl-C interrupt _without_
        /// immediately sending a stop reason, and instead deferring the stop
        /// reason to some later point in the target's execution.
        ///
        /// _Suggestion_: If you're unsure which stop reason to report,
        /// [`BaseStopReason::Signal(Signal::SIGINT)`] is a sensible default.
        ///
        /// [`BaseStopReason::Signal(Signal::SIGINT)`]:
        /// crate::stub::BaseStopReason::Signal
        fn on_interrupt(
            target: &mut Self::Target,
        ) -> Result<Option<Self::StopReason>, <Self::Target as Target>::Error>;
    }

    /// Returned by the `wait_for_stop_reason` closure in
    /// [`GdbStub::run_blocking`]
    pub enum Event<StopReason> {
        /// GDB Client sent data while the target was running.
        IncomingData(u8),
        /// The target has stopped.
        TargetStopped(StopReason),
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
    /// [`GdbStubStateMachine`](state_machine::GdbStubStateMachine) API
    /// directly.
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
                state_machine::GdbStubStateMachine::Idle(mut gdb) => {
                    // needs more data, so perform a blocking read on the connection
                    let byte = gdb.borrow_conn().read().map_err(Error::ConnectionRead)?;
                    gdb.incoming_data(target, byte)?
                }

                state_machine::GdbStubStateMachine::Disconnected(gdb) => {
                    // run_blocking keeps things simple, and doesn't expose a way to re-use the
                    // state machine
                    break Ok(gdb.get_reason());
                }

                state_machine::GdbStubStateMachine::CtrlCInterrupt(gdb) => {
                    // defer to the implementation on how it wants to handle the interrupt
                    let stop_reason = E::on_interrupt(target).map_err(Error::TargetError)?;
                    gdb.interrupt_handled(target, stop_reason)?
                }

                state_machine::GdbStubStateMachine::Running(mut gdb) => {
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

            if !support_software_breakpoints && !target.guard_rail_implicit_sw_breakpoints() {
                return Err(Error::ImplicitSwBreakpoints);
            }
        }

        // Check how the target's arch handles single stepping
        {
            use crate::arch::SingleStepGdbBehavior;
            use crate::target::ext::base::ResumeOps;

            if let Some(ops) = target.base_ops().resume_ops() {
                let support_single_step = match ops {
                    ResumeOps::SingleThread(ops) => ops.support_single_step().is_some(),
                    ResumeOps::MultiThread(ops) => ops.support_single_step().is_some(),
                };

                let behavior = target.guard_rail_single_step_gdb_behavior();

                let return_error = match behavior {
                    SingleStepGdbBehavior::Optional => false,
                    SingleStepGdbBehavior::Required => !support_single_step,
                    SingleStepGdbBehavior::Ignored => support_single_step,
                };

                if return_error {
                    return Err(Error::SingleStepGdbBehavior(behavior));
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
