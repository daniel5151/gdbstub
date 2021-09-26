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
    /// This method provides a quick and easy way to get up and running with
    /// `gdbstub`, and can be used as a _stepping-stone_ towards the
    /// fully-featured [state-machine](`state_machine::GdbStubStateMachine`)
    /// based interface.
    ///
    /// `GdbStub::run` will only return once the GDB client closes the debugging
    /// session, or if the target halts.
    ///
    /// # Implementing `read_byte`
    ///
    /// `read_byte` must fetch a single byte from the underlying `Connection`
    /// using a _blocking read_. The precise mechanism of how to read a
    /// single byte depends on the type of `Connection` being used.
    ///
    /// If you're using the standard library's `TcpStream` or `UnixStream`,
    /// `gdbstub` provides implementations of
    /// [`ConnectionExt::read`](crate::ConnectionExt::read) for these types,
    /// which perfectly matches the function signature of `read_byte`:
    ///
    /// ```rust
    /// # use gdbstub::target::Target;
    /// # use gdbstub::{ConnectionExt, GdbStubError, GdbStub};
    /// # fn run_debugger<T: Target, C: ConnectionExt>(
    /// #     mut target: T,
    /// #     mut gdb: GdbStub<'_, T, C>,
    /// # ) -> Result<(), GdbStubError<T::Error, C::Error>> {
    /// gdb.run(&mut target, ConnectionExt::read);
    /// # unimplemented!()
    /// # }
    /// ```
    ///
    /// As another example, if the `Connection` implements `std::io::Read`,
    /// one way to implement `read_byte` is as follows:
    ///
    /// ```rust
    /// use std::io::Read;
    /// # fn foo<C>(c: C) -> impl FnMut(&mut C) -> Result<u8, C::Error>
    /// # where C: gdbstub::Connection<Error = std::io::Error> + Read
    /// # {
    /// |conn| conn.bytes().next().unwrap()
    /// # }
    /// ```
    ///
    /// # Limitations of `run`, and when to switch to `run_state_machine`
    ///
    /// As you continue to flesh-out your target implementation, you'll soon run
    /// into some major limitations of the `GdbStub::run` API:
    ///
    /// ## No support of deferred stop reasons.
    ///
    /// If a target attempts to use deferred stop reasons while running under
    /// `GdbStub::run`, this API will return [`Error::CannotReturnDefer`] at
    /// runtime!
    ///
    /// ## Handling GDB Ctrl-C interrupts
    ///
    /// A target that doesn't use deferred stop reasons will need to _busy poll_
    /// for incoming GDB interrupts using the callback passed to `resume`. While
    /// conceptually simple, busy-polling is neither elegant nor efficient, as
    /// most transports support some kind of efficient "waiting" mechanism
    /// (e.g: epoll/select/kqueue, async/await, etc...)
    ///
    /// Using the more advanced `GdbStubStateMachine` API (alongside deferred
    /// stop reasons) makes it possible to lift the responsibility of checking
    /// for GDB Ctrl-C interrupts out of the target's `resume` implementation
    /// (i.e: polling the callback function), and into the top-level `gdbstub`
    /// event loop.
    ///
    /// A key consequence of lifting this responsibility up the call-stack is
    /// that the `gdbstub` event loop knows the _concrete_ `Connection` type
    /// being used, enabling implementations to leverage whatever
    /// transport-specific efficient waiting mechanism it exposes. Compare this
    /// with polling for interrupts in the target's `resume` implementation,
    /// where the method _doesn't_ have access to the the concrete `Connection`
    /// type being used.
    ///
    /// ## Driving `gdbstub` in an event loop / via interrupt handlers
    ///
    /// The `read_byte` closure used by `GdbStub::run` is a _blocking_ API,
    /// which will block the thread that that GDB server is running on.
    /// Conceptually, this API will "pull" data from the connection whenever it
    /// requires it.
    ///
    /// This blocking behavior can be a non-starter when integrating `gdbstub`
    /// in certain projects, such as `no_std` projects using `gdbstub` to debug
    /// code at the bare-metal. In these scenarios, it may not be possible to
    /// "block" the current thread of execution, as doing so would effectively
    /// block the entire machine.
    ///
    /// `GdbStubStateMachine` provides an alternative "push" based API, whereby
    /// the implementation can provide data to `gdbstub` as it becomes available
    /// (e.g: via a UART interrupt handler), and having `gdbstub` "react"
    /// whenever a complete packet is received.
    pub fn run(
        &mut self,
        target: &mut T,
        mut read_byte: impl FnMut(&mut C) -> Result<u8, C::Error>,
    ) -> Result<DisconnectReason, Error<T::Error, C::Error>> {
        // destructure Self to avoid borrow checker issues when invoking
        // `RecvPacketBlocking::recv`
        let Self {
            conn,
            packet_buffer,
            inner,
        } = self;

        conn.on_session_start().map_err(Error::ConnectionRead)?;

        loop {
            use crate::protocol::recv_packet::{RecvPacketBlocking, RecvPacketError};

            let buf = match RecvPacketBlocking::new().recv(packet_buffer, || read_byte(conn)) {
                Err(RecvPacketError::Capacity) => return Err(Error::PacketBufferOverflow),
                Err(RecvPacketError::Connection(e)) => return Err(Error::ConnectionWrite(e)),
                Ok(buf) => buf,
            };

            let packet = Packet::from_buf(target, buf).map_err(Error::PacketParse)?;
            match inner.handle_packet(target, conn, packet)? {
                State::Pump => {}
                State::Disconnect(reason) => return Ok(reason),
                State::DeferredStopReason => return Err(Error::CannotReturnDefer),
                // This arm will never get hit, as client will only ever send interrupt packets when
                // the target is running.
                State::CtrlCInterrupt => {
                    log::error!("Unexpected interrupt packet!");
                    return Err(Error::PacketUnexpected);
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
    /// as such may return a connection error.
    pub fn run_state_machine(
        mut self,
    ) -> Result<state_machine::GdbStubStateMachine<'a, T, C>, Error<T::Error, C::Error>> {
        self.conn
            .on_session_start()
            .map_err(Error::ConnectionRead)?;

        Ok(state_machine::GdbStubStateMachineInner::from_plain_gdbstub(self).into())
    }
}

pub use state_machine::GdbStubStateMachine;

/// State-machine interface to `GdbStub`.
///
/// TODO: more docs. also discuss the typestate token API...
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

    /// Methods which can be called from the [`GdbStubStateMachine::Pump`]
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

    /// Methods which can be called from the
    /// [`GdbStubStateMachine::DeferredStopReason`] state.
    impl<'a, T: Target, C: Connection> GdbStubStateMachineInner<'a, state::DeferredStopReason, T, C> {
        /// Report a target stop reason back to GDB.
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
            res.flush()?;

            Ok((self.transition::<state::Pump>().into(), event))
        }

        /// Pass a byte to the GDB stub.
        // DEVNOTE: unlike the `pump` method in the `state::Pump` state, this method
        // doesn't transition to `state::Pump`, as the client is still waiting for the
        // target to report a stop reason.
        pub fn pump(
            mut self,
            target: &mut T,
            byte: u8,
        ) -> Result<(GdbStubStateMachine<'a, T, C>, Event), Error<T::Error, C::Error>> {
            let packet_buffer = match self.recv_packet.pump(&mut self.packet_buffer, byte)? {
                Some(buf) => buf,
                None => return Ok((self.into(), Event::None)),
            };

            let packet = Packet::from_buf(target, packet_buffer).map_err(Error::PacketParse)?;
            let state = self.inner.handle_packet(target, &mut self.conn, packet)?;
            match state {
                State::Pump => Ok((self.into(), Event::None)),
                State::Disconnect(reason) => Ok((self.into(), Event::Disconnect(reason))),
                State::DeferredStopReason => Ok((self.into(), Event::None)),
                State::CtrlCInterrupt => Ok((self.into(), Event::CtrlCInterrupt)),
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
        }
    }
}
