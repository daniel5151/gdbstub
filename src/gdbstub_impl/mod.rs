use core::marker::PhantomData;

use managed::ManagedSlice;

use crate::arch::Arch;
use crate::common::*;
use crate::connection::{Connection, ConnectionExt};
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
    TargetTerminated(u8),
    /// GDB issued a disconnect command
    Disconnect,
    /// GDB issued a kill command
    Kill,
}

/// Types and traits related to the [`GdbStub::run`] interface.
pub mod gdbstub_run_blocking {
    use super::*;

    use crate::connection::ConnectionExt;

    /// A set of user-provided methods required to run a GDB debugging session
    /// using the [`GdbStub::run`](super::GdbStub::run) method.
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
        /// called. The implementation should block until the target reports a
        /// stop reason, or if detects that the GDB client has sent additional
        /// data over the connection.
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

        /// Invoked when the GDB client sends a Ctrl-C interrupt. Returns the
        /// stop reason that should be reported back to the GDB client, or
        /// `None` if the interrupt should be ignored.
        ///
        /// _Suggestion_: If you're unsure which stop reason you should report,
        /// [`ThreadStopReason::GdbCtrlCInterrupt`] is a reasonable default.
        /// Under the hood, this is equivalent to returning
        /// [`ThreadStopReason::Signal(5)`](ThreadStopReason::Signal), or a
        /// SIGTRAP.
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

    /// Exnteds [`Target`] with a few additional methods required by the
    /// `GdbStub::run` method.
    ///
    /// If you are interested in using `gdbstub` without blocking (e.g: via
    /// async/await, unix polling, from an interrupt handler, etc...) you'll
    /// need to interface with the
    /// [`GdbStubStateMachine`](super::GdbStubStateMachine) API directly.
    pub trait TargetRun: Target {
        /// Connection being used to drive the target.
        type Connection: ConnectionExt;
    }

    /// Returned by the `wait_for_stop_reason` closure in
    /// [`GdbStub::run`](super::GdbStub::run)
    pub enum Event<U> {
        /// GDB Client sent data while the target was running.
        IncomingData(u8),
        /// The target has stopped.
        TargetStopped(ThreadStopReason<U>),
    }

    /// Error value returned by the `wait_for_stop_reason` closure in
    /// [`GdbStub::run`](super::GdbStub::run)
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
    /// [`gdbstub_run_blocking::BlockingEventLoop`], which is a simplified set
    /// of methods describing how to drive the target.
    ///
    /// `GdbStub::run` returns once the GDB client closes the debugging session,
    /// or if the target halts.
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
        E: gdbstub_run_blocking::BlockingEventLoop<Target = T, Connection = C>,
    {
        let mut gdb = self.run_state_machine()?;
        loop {
            gdb = match gdb {
                GdbStubStateMachine::Pump(mut gdb) => {
                    // needs more data, so perform a blocking read on the connection
                    let byte = gdb.borrow_conn().read().map_err(Error::ConnectionRead)?;

                    let (gdb, disconnect_reason) = gdb.pump(target, byte)?;

                    if let Some(disconnect_reason) = disconnect_reason {
                        break Ok(disconnect_reason);
                    }

                    gdb
                }

                GdbStubStateMachine::DeferredStopReason(mut gdb) => {
                    use gdbstub_run_blocking::{
                        Event as BlockingEventLoopEvent, WaitForStopReasonError,
                    };
                    use state_machine::Event;

                    // block waiting for the target to return a stop reason
                    let event = E::wait_for_stop_reason(target, gdb.borrow_conn());
                    match event {
                        Ok(BlockingEventLoopEvent::TargetStopped(stop_reason)) => {
                            match gdb.deferred_stop_reason(target, stop_reason)? {
                                (_, Some(disconnect_reason)) => break Ok(disconnect_reason),
                                (gdb, None) => gdb,
                            }
                        }

                        Ok(BlockingEventLoopEvent::IncomingData(byte)) => {
                            let (gdb, event) = gdb.pump(target, byte)?;

                            match event {
                                Event::None => gdb.into(),
                                Event::Disconnect(disconnect_reason) => {
                                    break Ok(disconnect_reason)
                                }
                                Event::CtrlCInterrupt => {
                                    // defer to the implementation on how it wants to handle the
                                    // interrupt...
                                    let stop_reason =
                                        E::on_interrupt(target).map_err(Error::TargetError)?;
                                    // if the target wants to handle the interrupt, report the
                                    // stop reason
                                    if let Some(stop_reason) = stop_reason {
                                        match gdb.deferred_stop_reason(target, stop_reason)? {
                                            (_, Some(disconnect_reason)) => {
                                                break Ok(disconnect_reason)
                                            }
                                            (gdb, None) => gdb,
                                        }
                                    } else {
                                        gdb.into()
                                    }
                                }
                            }
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
    ///
    /// Note: This method will invoke `Connection::on_session_start`, and
    /// as such, it may return a [`GdbStubError::ConnectionRead`].
    pub fn run_state_machine(
        mut self,
    ) -> Result<state_machine::GdbStubStateMachine<'a, T, C>, Error<T::Error, C::Error>> {
        self.conn
            .on_session_start()
            .map_err(Error::ConnectionRead)?;

        Ok(state_machine::GdbStubStateMachineInner::from_plain_gdbstub(self).into())
    }
}

/// State-machine interface to `GdbStub`.
///
/// TODO: more docs. provide an example of how to use this API.
///
/// TODO: add docs to top-level `lib.rs` that point folks at this API.
pub mod state_machine {
    use super::*;

    use crate::protocol::recv_packet::RecvPacketStateMachine;

    /// State-machine interface to `GdbStub`, supporting advanced features such
    /// as deferred stop reasons and incremental packet processing.
    ///
    /// See the [module level documentation](self) for more details.
    pub enum GdbStubStateMachine<'a, T, C>
    where
        T: Target,
        C: Connection,
    {
        /// The target is completely stopped, and the GDB stub is waiting for
        /// additional input.
        Pump(GdbStubStateMachineInner<'a, state::Pump, T, C>),
        /// The target is currently running, and the GDB client is waiting for
        /// the target to report a stop reason.
        ///
        /// Note that the client may still send packets to the target
        /// (e.g: to trigger a Ctrl-C interrupt).
        DeferredStopReason(GdbStubStateMachineInner<'a, state::DeferredStopReason, T, C>),
    }

    /// Zero-sized typestates.
    ///
    /// The types in this module are used to parameterize instances of
    /// `GdbStubStateMachineInner`, thereby enforcing that certain API methods
    /// can only be called while the stub is in a certain state.
    pub mod state {
        /// ZST typestate corresponding to the "Pump" state.
        pub enum Pump {}

        /// ZST typestate corresponding to the "DeferredStopReason" state.
        pub enum DeferredStopReason {}
    }

    /// Internal helper macro to convert between a particular inner state into
    /// its corresponding `GdbStubStateMachine` variant.
    macro_rules! impl_from_inner {
        ($state:ident) => {
            impl<'a, T, C> From<GdbStubStateMachineInner<'a, state::$state, T, C>>
                for GdbStubStateMachine<'a, T, C>
            where
                T: Target,
                C: Connection,
            {
                fn from(inner: GdbStubStateMachineInner<'a, state::$state, T, C>) -> Self {
                    GdbStubStateMachine::$state(inner)
                }
            }
        };
    }

    impl_from_inner!(Pump);
    impl_from_inner!(DeferredStopReason);

    /// Internal helper trait to cut down on boilerplate required to transition
    /// between states.
    trait Transition<'a, T, C>
    where
        T: Target,
        C: Connection,
    {
        fn transition<S>(self) -> GdbStubStateMachineInner<'a, S, T, C>;
    }

    impl<'a, S1, T, C> Transition<'a, T, C> for GdbStubStateMachineInner<'a, S1, T, C>
    where
        T: Target,
        C: Connection,
    {
        #[inline(always)]
        fn transition<S>(self) -> GdbStubStateMachineInner<'a, S, T, C> {
            GdbStubStateMachineInner {
                conn: self.conn,
                packet_buffer: self.packet_buffer,
                recv_packet: self.recv_packet,
                inner: self.inner,
                _state: PhantomData,
            }
        }
    }

    /// Core state machine implementation which is parameterized by various
    /// [states](state). Can be converted back into the appropriate
    /// [`GdbStubStateMachine`] variant via [`Into::into`].
    pub struct GdbStubStateMachineInner<'a, S, T: Target, C: Connection> {
        conn: C,
        packet_buffer: ManagedSlice<'a, u8>,
        recv_packet: RecvPacketStateMachine,
        inner: GdbStubImpl<T, C>,
        _state: PhantomData<S>,
    }

    /// Methods which can be called regardless of the current state.
    impl<'a, S, T: Target, C: Connection> GdbStubStateMachineInner<'a, S, T, C> {
        /// Return a mutable reference to the underlying connection.
        pub fn borrow_conn(&mut self) -> &mut C {
            &mut self.conn
        }
    }

    /// Methods which can only be called from the [`GdbStubStateMachine::Pump`]
    /// state.
    impl<'a, T: Target, C: Connection> GdbStubStateMachineInner<'a, state::Pump, T, C> {
        pub(crate) fn from_plain_gdbstub(
            stub: GdbStub<'a, T, C>,
        ) -> GdbStubStateMachineInner<'a, state::Pump, T, C> {
            GdbStubStateMachineInner {
                conn: stub.conn,
                packet_buffer: stub.packet_buffer,
                recv_packet: RecvPacketStateMachine::new(),
                inner: stub.inner,
                _state: PhantomData,
            }
        }

        /// Pass a byte to the GDB stub.
        pub fn pump(
            mut self,
            target: &mut T,
            byte: u8,
        ) -> Result<
            (GdbStubStateMachine<'a, T, C>, Option<DisconnectReason>),
            Error<T::Error, C::Error>,
        > {
            let packet_buffer = match self.recv_packet.pump(&mut self.packet_buffer, byte)? {
                Some(buf) => buf,
                None => return Ok((self.into(), None)),
            };

            let packet = Packet::from_buf(target, packet_buffer).map_err(Error::PacketParse)?;
            let state = self.inner.handle_packet(target, &mut self.conn, packet)?;
            match state {
                State::Pump => Ok((self.into(), None)),
                State::Disconnect(reason) => Ok((self.into(), Some(reason))),
                State::DeferredStopReason => {
                    Ok((self.transition::<state::DeferredStopReason>().into(), None))
                }
                // This arm will never get hit, as client will only ever send interrupt packets when
                // the target is running.
                State::CtrlCInterrupt => {
                    log::error!("Unexpected interrupt packet!");
                    Err(Error::PacketUnexpected)
                }
            }
        }
    }

    /// Methods which can only be called from the
    /// [`GdbStubStateMachine::DeferredStopReason`] state.
    impl<'a, T: Target, C: Connection> GdbStubStateMachineInner<'a, state::DeferredStopReason, T, C> {
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
        pub fn deferred_stop_reason(
            mut self,
            target: &mut T,
            reason: ThreadStopReason<<T::Arch as Arch>::Usize>,
        ) -> Result<
            (GdbStubStateMachine<'a, T, C>, Option<DisconnectReason>),
            Error<T::Error, C::Error>,
        > {
            let mut res = ResponseWriter::new(&mut self.conn);
            let event = match self.inner.finish_exec(&mut res, target, reason)? {
                ext::FinishExecStatus::Handled => None,
                ext::FinishExecStatus::Disconnect(reason) => Some(reason),
            };
            res.flush()?;

            Ok((self.transition::<state::Pump>().into(), event))
        }

        /// Pass a byte to the GDB stub.
        ///
        /// NOTE: unlike the `pump` method in the `state::Pump` state, this
        /// method does not perform any state transitions, and will  return a
        /// `GdbStubStateMachineInner` in the `state::DeferredStopReason` state.
        pub fn pump(
            mut self,
            target: &mut T,
            byte: u8,
        ) -> Result<(Self, Event), Error<T::Error, C::Error>> {
            let packet_buffer = match self.recv_packet.pump(&mut self.packet_buffer, byte)? {
                Some(buf) => buf,
                None => return Ok((self, Event::None)),
            };

            let packet = Packet::from_buf(target, packet_buffer).map_err(Error::PacketParse)?;
            let state = self.inner.handle_packet(target, &mut self.conn, packet)?;
            match state {
                State::Pump => Ok((self, Event::None)),
                State::Disconnect(reason) => Ok((self, Event::Disconnect(reason))),
                State::DeferredStopReason => Ok((self, Event::None)),
                State::CtrlCInterrupt => Ok((self, Event::CtrlCInterrupt)),
            }
        }
    }

    /// Events which may occur after calling `pump()` on a
    /// [`GdbStubStateMachine::DeferredStopReason`].
    pub enum Event {
        /// Nothing happened.
        None,
        /// The client has triggered a disconnect.
        Disconnect(DisconnectReason),
        /// The client has sent a Ctrl-C interrupt.
        ///
        /// Please note the following GDB docs:
        ///
        /// > Stubs are not required to recognize these interrupt mechanisms and
        /// the precise meaning associated with receipt of the interrupt is
        /// implementation defined.
        /// >
        /// > If the target supports debugging of multiple threads and/or
        /// processes, it should attempt to interrupt all currently-executing
        /// threads and processes. If the stub is successful at interrupting the
        /// running program, it should send one of the stop reply packets (see
        /// Stop Reply Packets) to GDB as a result of successfully stopping the
        /// program in all-stop mode, and a stop reply for each stopped thread
        /// in non-stop mode.
        /// >
        /// > Interrupts received while the program is stopped are queued and
        /// the program will be interrupted when it is resumed next time.
        CtrlCInterrupt,
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
                if !(target.extended_mode().is_none() && is_kill) {
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
            Command::Unknown(cmd) => {
                info!("Unknown command: {:?}", core::str::from_utf8(cmd));
                Ok(HandlerStatus::Handled)
            }
            // `handle_X` methods are defined in the `ext` module
            Command::Base(cmd) => self.handle_base(res, target, cmd),
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
        }
    }
}
