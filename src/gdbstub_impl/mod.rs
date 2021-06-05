use core::marker::PhantomData;

use managed::ManagedSlice;

use crate::common::*;
use crate::connection::Connection;
use crate::protocol::{commands::Command, Packet, ResponseWriter, SpecificIdKind};
use crate::target::Target;
use crate::SINGLE_THREAD_TID;

mod builder;
mod error;
mod ext;
mod target_result_ext;

pub use builder::{GdbStubBuilder, GdbStubBuilderError};
pub use error::GdbStubError;

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

    /// Starts a GDB remote debugging session.
    ///
    /// Returns once the GDB client closes the debugging session, or if the
    /// target halts.
    pub fn run(&mut self, target: &mut T) -> Result<DisconnectReason, Error<T::Error, C::Error>> {
        self.conn
            .on_session_start()
            .map_err(Error::ConnectionRead)?;

        loop {
            use crate::protocol::recv_packet::{RecvPacketBlocking, RecvPacketError};

            let Self {
                conn,
                packet_buffer,
                ..
            } = self;

            let buf = match RecvPacketBlocking::new().recv(packet_buffer, || conn.read()) {
                Err(RecvPacketError::Capacity) => return Err(Error::PacketBufferOverflow),
                Err(RecvPacketError::Connection(e)) => return Err(Error::ConnectionWrite(e)),
                Ok(buf) => buf,
            };

            let packet = Packet::from_buf(target, buf).map_err(Error::PacketParse)?;
            match self.inner.handle_packet(target, &mut self.conn, packet)? {
                State::Pump => {}
                State::Disconnect(reason) => return Ok(reason),
                State::DeferredStopReason => return Err(Error::CannotReturnDefer),
            }
        }
    }

    /// Starts a GDB remote debugging session, and convert this instance of
    /// `GdbStub` into a
    /// [`GdbStubStateMachine`](state_machine::GdbStubStateMachine) that is
    /// ready to receive data.
    ///
    /// Note: This method will invoke `Connection::on_session_start`, and
    /// as such may return a connection error.
    #[allow(clippy::type_complexity)]
    pub fn run_state_machine(
        mut self,
    ) -> Result<state_machine::GdbStubStateMachine<'a, T, C>, Error<T::Error, C::Error>> {
        self.conn
            .on_session_start()
            .map_err(Error::ConnectionRead)?;

        Ok(state_machine::GdbStubStateMachineInner::from_plain_gdbstub(self).into())
    }
}

/// More complex state-machine based interface to `GdbStub` which supports
/// deferred stop reason reporting and incremental packet processing.
///
/// TODO: more docs. also discuss the typestate token API...
///
/// TODO: add docs to top-level `lib.rs` that point folks at this API.
#[allow(clippy::type_complexity)]
pub mod state_machine {
    use super::*;

    use crate::protocol::recv_packet::RecvPacketStateMachine;

    /// Wrapper around [`GdbStubStateMachineInner`] which encapsulates all
    /// possible state machine variants.
    pub enum GdbStubStateMachine<'a, T, C>
    where
        T: Target,
        C: Connection,
    {
        /// Stub is waiting for target to report a stop reason
        DeferredStopReason(GdbStubStateMachineInner<'a, state::DeferredStopReason, T, C>),
        /// Stub is waiting for additional input
        Pump(GdbStubStateMachineInner<'a, state::Pump, T, C>),
    }

    /// Zero-sized typestates.
    ///
    /// The types in this module are used to parameterize instances of
    /// `GdbStubStateMachineInner`, thereby enforcing that certain API methods
    /// can only be called while the stub is in a certain state.
    pub mod state {
        /// ZST typestate corresponding to the "DeferredStopReason" state.
        pub enum DeferredStopReason {}

        /// ZST typestate corresponding to the "Pump" state.
        pub enum Pump {}
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
    /// [states](state).
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
            }
        }
    }

    impl<'a, T: Target, C: Connection> GdbStubStateMachineInner<'a, state::DeferredStopReason, T, C> {
        /// Report a deferred target stop reason back to GDB.
        pub fn deferred_stop_reason(
            mut self,
            target: &mut T,
            reason: crate::target::ext::base::multithread::ThreadStopReason<
                <T::Arch as crate::arch::Arch>::Usize,
            >,
        ) -> Result<
            (GdbStubStateMachine<'a, T, C>, Option<DisconnectReason>),
            Error<T::Error, C::Error>,
        > {
            let mut res = ResponseWriter::new(&mut self.conn);
            let event = match self.inner.finish_exec(&mut res, target, reason)? {
                ext::FinishExecStatus::Handled => None,
                ext::FinishExecStatus::Disconnect(reason) => Some(reason),
            };

            Ok((self.transition::<state::Pump>().into(), event))
        }
    }
}

enum State {
    Pump,
    DeferredStopReason,
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
                let mut res = ResponseWriter::new(conn);
                res.write_str("S05")?;
                res.flush()?;
                Ok(State::Pump)
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
                // cmd must be ASCII, as the slice originated from a PacketBuf, which checks for
                // ASCII as part of the initial validation.
                info!("Unknown command: {}", core::str::from_utf8(cmd).unwrap());
                Ok(HandlerStatus::Handled)
            }
            // `handle_X` methods are defined in the `ext` module
            Command::Base(cmd) => self.handle_base(res, target, cmd),
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
        }
    }
}
