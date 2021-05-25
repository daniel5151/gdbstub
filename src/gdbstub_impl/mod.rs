use core::marker::PhantomData;

use managed::ManagedSlice;

use crate::common::*;
use crate::connection::Connection;
use crate::protocol::recv_packet::{RecvPacketBlocking, RecvPacketStateMachine};
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
    state: GdbStubImpl<T, C>,
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
        self.state
            .run(target, &mut self.conn, &mut self.packet_buffer)
    }

    /// Starts a GDB remote debugging session, and convert this instance of
    /// `GdbStub` into a [`GdbStubStateMachine`].
    ///
    /// Note: This method will invoke `Connection::on_session_start`, and
    /// as such may return a connection error.
    pub fn run_state_machine(
        mut self,
    ) -> Result<GdbStubStateMachine<'a, T, C>, Error<T::Error, C::Error>> {
        self.conn
            .on_session_start()
            .map_err(Error::ConnectionRead)?;

        Ok(GdbStubStateMachine {
            conn: self.conn,
            packet_buffer: self.packet_buffer,
            recv_packet: RecvPacketStateMachine::new(),
            state: self.state,
        })
    }
}

/// A variant of [`GdbStub`] which parses incoming packets using an asynchronous
/// state machine.
///
/// TODO: more docs
pub struct GdbStubStateMachine<'a, T: Target, C: Connection> {
    conn: C,
    packet_buffer: ManagedSlice<'a, u8>,
    recv_packet: RecvPacketStateMachine,
    state: GdbStubImpl<T, C>,
}

impl<'a, T: Target, C: Connection> GdbStubStateMachine<'a, T, C> {
    /// Pass a byte to the `gdbstub` packet parser.
    ///
    /// Returns a `Some(DisconnectReason)` if the GDB client
    pub fn pump(
        &mut self,
        target: &mut T,
        byte: u8,
    ) -> Result<Option<DisconnectReason>, Error<T::Error, C::Error>> {
        let packet_buffer = match self.recv_packet.pump(&mut self.packet_buffer, byte)? {
            Some(buf) => buf,
            None => return Ok(None),
        };

        let packet = Packet::from_buf(target, packet_buffer).map_err(Error::PacketParse)?;
        self.state.handle_packet(target, &mut self.conn, packet)
    }

    /// Return a mutable reference to the underlying connection.
    pub fn borrow_conn(&mut self) -> &mut C {
        &mut self.conn
    }
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

    fn run(
        &mut self,
        target: &mut T,
        conn: &mut C,
        packet_buffer: &mut ManagedSlice<u8>,
    ) -> Result<DisconnectReason, Error<T::Error, C::Error>> {
        conn.on_session_start().map_err(Error::ConnectionRead)?;

        loop {
            use crate::protocol::recv_packet::RecvPacketError;
            let packet_buffer = match RecvPacketBlocking::new().recv(packet_buffer, || conn.read())
            {
                Err(RecvPacketError::Capacity) => return Err(Error::PacketBufferOverflow),
                Err(RecvPacketError::Connection(e)) => return Err(Error::ConnectionWrite(e)),
                Ok(buf) => buf,
            };

            let packet = Packet::from_buf(target, packet_buffer).map_err(Error::PacketParse)?;
            if let Some(disconnect_reason) = self.handle_packet(target, conn, packet)? {
                return Ok(disconnect_reason);
            }
        }
    }

    fn handle_packet(
        &mut self,
        target: &mut T,
        conn: &mut C,
        packet: Packet<'_>,
    ) -> Result<Option<DisconnectReason>, Error<T::Error, C::Error>> {
        match packet {
            Packet::Ack => {}
            Packet::Nack => return Err(Error::ClientSentNack),
            Packet::Interrupt => {
                debug!("<-- interrupt packet");
                let mut res = ResponseWriter::new(conn);
                res.write_str("S05")?;
                res.flush()?;
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

                return Ok(disconnect_reason);
            }
        };

        Ok(None)
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
