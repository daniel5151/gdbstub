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
//! - Real-world projects using the API
//!     - The best example of this (at the time of writing) is the code at
//!     [`vmware-labs/node-replicated-kernel`](https://github.com/vmware-labs/node-replicated-kernel/blob/4326704aaf3c0052e614dcde2a788a8483224394/kernel/src/arch/x86_64/gdb/mod.rs#L106)
//!
//! If you have any questions, feel free to open a discussion thread over at the
//! [`gdbstub` GitHub repo](https://github.com/daniel5151/gdbstub/).
//!
//! [`BlockingEventLoop`]: super::run_blocking::BlockingEventLoop
//! [`GdbStub::run_blocking`]: super::GdbStub::run_blocking

use managed::ManagedSlice;

use crate::arch::Arch;
use crate::conn::Connection;
use crate::protocol::recv_packet::RecvPacketStateMachine;
use crate::protocol::{Packet, ResponseWriter};
use crate::stub::error::GdbStubError as Error;
use crate::stub::stop_reason::IntoStopReason;
use crate::target::Target;

use super::core_impl::{FinishExecStatus, GdbStubImpl, State};
use super::{DisconnectReason, GdbStub};

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

    use crate::stub::stop_reason::MultiThreadStopReason;

    // used internally when logging state transitions
    pub(crate) const MODULE_PATH: &str = concat!(module_path!(), "::");

    /// Typestate corresponding to the "Idle" state.
    #[non_exhaustive]
    pub struct Idle<T: Target> {
        pub(crate) deferred_ctrlc_stop_reason:
            Option<MultiThreadStopReason<<<T as Target>::Arch as Arch>::Usize>>,
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
    pub fn report_stop(
        mut self,
        target: &mut T,
        reason: impl IntoStopReason<T>,
    ) -> Result<GdbStubStateMachine<'a, T, C>, Error<T::Error, C::Error>> {
        let mut res = ResponseWriter::new(&mut self.i.conn, target.use_rle());
        let event = self.i.inner.finish_exec(&mut res, target, reason.into())?;
        res.flush()?;

        Ok(match event {
            FinishExecStatus::Handled => self
                .transition(state::Idle {
                    deferred_ctrlc_stop_reason: None,
                })
                .into(),
            FinishExecStatus::Disconnect(reason) => {
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
    /// Acknowledge the Ctrl-C interrupt.
    ///
    /// Passing `None` as a stop reason will return the state machine to
    /// whatever state it was in pre-interruption, without immediately returning
    /// a stop reason.
    ///
    /// Depending on how the target is implemented, it may or may not make sense
    /// to immediately return a stop reason as part of handling the Ctrl-C
    /// interrupt. e.g: in some cases, it may be better to send the target a
    /// signal upon receiving a Ctrl-C interrupt _without_ immediately sending a
    /// stop reason, and instead deferring the stop reason to some later point
    /// in the target's execution.
    ///
    /// Some notes on handling Ctrl-C interrupts:
    ///
    /// - Stubs are not required to recognize these interrupt mechanisms, and
    ///   the precise meaning associated with receipt of the interrupt is
    ///   implementation defined.
    /// - If the target supports debugging of multiple threads and/or processes,
    ///   it should attempt to interrupt all currently-executing threads and
    ///   processes.
    /// - If the stub is successful at interrupting the running program, it
    ///   should send one of the stop reply packets (see Stop Reply Packets) to
    ///   GDB as a result of successfully stopping the program
    pub fn interrupt_handled(
        self,
        target: &mut T,
        stop_reason: Option<impl IntoStopReason<T>>,
    ) -> Result<GdbStubStateMachine<'a, T, C>, Error<T::Error, C::Error>> {
        if self.state.from_idle {
            // target is stopped - we cannot report the stop reason yet
            Ok(self
                .transition(state::Idle {
                    deferred_ctrlc_stop_reason: stop_reason.map(Into::into),
                })
                .into())
        } else {
            // target is running - we can immediately report the stop reason
            let gdb = self.transition(state::Running {});
            match stop_reason {
                Some(reason) => gdb.report_stop(target, reason),
                None => Ok(gdb.into()),
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
