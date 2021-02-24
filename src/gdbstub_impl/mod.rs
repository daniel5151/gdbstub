use core::marker::PhantomData;

#[cfg(feature = "alloc")]
use alloc::collections::BTreeMap;

use managed::ManagedSlice;

use crate::common::*;
use crate::connection::Connection;
use crate::protocol::{commands::Command, Packet, ResponseWriter, SpecificIdKind};
use crate::target::Target;
use crate::util::managed_vec::ManagedVec;
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
    /// Target Halted
    TargetHalted,
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
    /// For fine-grained control over various `GdbStub` options, use the
    /// [`builder()`](GdbStub::builder) method instead.
    ///
    /// _Note:_ `new` is only available when the `alloc` feature is enabled.
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
}

struct GdbStubImpl<T: Target, C: Connection> {
    _target: PhantomData<T>,
    _connection: PhantomData<C>,

    current_mem_tid: Tid,
    current_resume_tid: SpecificIdKind,
    no_ack_mode: bool,

    // Used to track which Pids were attached to / spawned when running in extended mode.
    //
    // An empty `BTreeMap<Pid, bool>` is only 24 bytes (on 64-bit systems), and doesn't allocate
    // until the first element is inserted, so it should be fine to include it as part of the main
    // state structure whether or not extended mode is actually being used.
    #[cfg(feature = "alloc")]
    attached_pids: BTreeMap<Pid, bool>,
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

            // NOTE: current_mem_tid is never queried prior to being set by the GDB client (via the
            // 'H' packet), so it's fine to use a dummy value here.
            //
            // Even if the GDB client is acting strangely and doesn't overwrite it, the target will
            // simply return a non-fatal error, which is totally fine.
            current_mem_tid: SINGLE_THREAD_TID,
            current_resume_tid: SpecificIdKind::All,
            no_ack_mode: false,

            #[cfg(feature = "alloc")]
            attached_pids: BTreeMap::new(),
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
            match Self::recv_packet(conn, target, packet_buffer)? {
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
                        conn.write(b'+').map_err(Error::ConnectionRead)?;
                    }

                    let mut res = ResponseWriter::new(conn);
                    let disconnect = match self.handle_command(&mut res, target, command) {
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

                    // HACK: this could be more elegant...
                    if disconnect != Some(DisconnectReason::Kill) {
                        res.flush()?;
                    }

                    if let Some(disconnect_reason) = disconnect {
                        return Ok(disconnect_reason);
                    }
                }
            };
        }
    }

    fn recv_packet<'a>(
        conn: &mut C,
        target: &mut T,
        pkt_buf: &'a mut ManagedSlice<u8>,
    ) -> Result<Packet<'a>, Error<T::Error, C::Error>> {
        let header_byte = conn.read().map_err(Error::ConnectionRead)?;

        // Wrap the buf in a `ManagedVec` to keep the code readable.
        let mut buf = ManagedVec::new(pkt_buf);

        buf.clear();
        buf.push(header_byte)?;
        if header_byte == b'$' {
            // read the packet body
            loop {
                let c = conn.read().map_err(Error::ConnectionRead)?;
                buf.push(c)?;
                if c == b'#' {
                    break;
                }
            }
            // read the checksum as well
            buf.push(conn.read().map_err(Error::ConnectionRead)?)?;
            buf.push(conn.read().map_err(Error::ConnectionRead)?)?;
        }

        trace!(
            "<-- {}",
            core::str::from_utf8(buf.as_slice()).unwrap_or("<invalid packet>")
        );

        drop(buf);

        Packet::from_buf(target, pkt_buf.as_mut()).map_err(Error::PacketParse)
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
            Command::ExtendedMode(cmd) => self.handle_extended_mode(res, target, cmd),
            Command::MonitorCmd(cmd) => self.handle_monitor_cmd(res, target, cmd),
            Command::SectionOffsets(cmd) => self.handle_section_offsets(res, target, cmd),
        }
    }
}
