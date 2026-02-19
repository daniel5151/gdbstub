use crate::common::Signal;
use crate::common::Tid;
use crate::conn::Connection;
use crate::protocol::commands::Command;
use crate::protocol::Packet;
use crate::protocol::ResponseWriter;
use crate::protocol::SpecificIdKind;
use crate::stub::error::InternalError;
use crate::target::Target;
use crate::SINGLE_THREAD_TID;
use core::marker::PhantomData;

/// Common imports used by >50% of all extensions.
///
/// Do not clutter this prelude with types only used by a few extensions.
mod prelude {
    pub(super) use crate::conn::Connection;
    pub(super) use crate::internal::BeBytes;
    pub(super) use crate::protocol::ResponseWriter;
    pub(super) use crate::stub::core_impl::target_result_ext::TargetResultExt;
    pub(super) use crate::stub::core_impl::GdbStubImpl;
    pub(super) use crate::stub::core_impl::HandlerStatus;
    pub(super) use crate::stub::error::InternalError as Error;
    pub(super) use crate::target::Target;
}

mod auxv;
mod base;
mod breakpoints;
mod catch_syscalls;
mod exec_file;
mod extended_mode;
mod flash;
mod host_io;
mod libraries;
mod lldb_register_info;
mod memory_map;
mod monitor_cmd;
mod no_ack_mode;
mod resume;
mod reverse_exec;
mod section_offsets;
mod single_register_access;
mod target_xml;
mod thread_extra_info;
mod tracepoints;
mod x_lowcase_packet;
mod x_upcase_packet;

pub(crate) use resume::FinishExecStatus;

pub(crate) mod target_result_ext {
    use crate::stub::error::InternalError;
    use crate::target::TargetError;

    /// Extension trait to ease working with `TargetResult` in the GdbStub
    /// implementation.
    pub(super) trait TargetResultExt<V, T, C> {
        /// Encapsulates the boilerplate associated with handling
        /// `TargetError`s, such as bailing-out on Fatal errors, or
        /// returning response codes.
        fn handle_error(self) -> Result<V, InternalError<T, C>>;
    }

    impl<V, T, C> TargetResultExt<V, T, C> for Result<V, TargetError<T>> {
        fn handle_error(self) -> Result<V, InternalError<T, C>> {
            let code = match self {
                Ok(v) => return Ok(v),
                Err(TargetError::Fatal(e)) => return Err(InternalError::TargetError(e)),
                // Recoverable errors:
                // Error code 121 corresponds to `EREMOTEIO` lol
                Err(TargetError::NonFatal) => 121,
                Err(TargetError::Errno(code)) => code,
                #[cfg(feature = "std")]
                Err(TargetError::Io(e)) => e.raw_os_error().unwrap_or(121) as u8,
            };

            Err(InternalError::NonFatalError(code))
        }
    }
}

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

pub enum State {
    Pump,
    DeferredStopReason,
    CtrlCInterrupt,
    Disconnect(DisconnectReason),
}

pub(crate) struct GdbStubImpl<T: Target, C: Connection> {
    _target: PhantomData<T>,
    _connection: PhantomData<C>,

    current_mem_tid: Tid,
    current_resume_tid: SpecificIdKind,
    features: ProtocolFeatures,
}

pub enum HandlerStatus {
    Handled,
    NeedsOk,
    DeferredStopReason,
    Disconnect(DisconnectReason),
}

impl<T: Target, C: Connection> GdbStubImpl<T, C> {
    pub fn new() -> GdbStubImpl<T, C> {
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
            features: ProtocolFeatures::empty(),
        }
    }

    pub fn handle_packet(
        &mut self,
        target: &mut T,
        conn: &mut C,
        packet: Packet<'_>,
    ) -> Result<State, InternalError<T::Error, C::Error>> {
        match packet {
            Packet::Ack => Ok(State::Pump),
            Packet::Nack => Err(InternalError::ClientSentNack),
            Packet::Interrupt => {
                debug!("<-- interrupt packet");
                Ok(State::CtrlCInterrupt)
            }
            Packet::Command(command) => {
                // Acknowledge the command
                if !self.features.no_ack_mode() {
                    conn.write(b'+').map_err(InternalError::conn_write)?;
                }

                let mut res = ResponseWriter::new(conn, target.use_rle());
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
                    Err(InternalError::NonFatalError(code)) => {
                        res.write_str("E")?;
                        res.write_num(code)?;
                        None
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
        res: &mut ResponseWriter<'_, C>,
        target: &mut T,
        cmd: Command<'_>,
    ) -> Result<HandlerStatus, InternalError<T::Error, C::Error>> {
        match cmd {
            // `handle_X` methods are defined in the `ext` module
            Command::Base(cmd) => self.handle_base(res, target, cmd),
            Command::TargetXml(cmd) => self.handle_target_xml(res, target, cmd),
            Command::Resume(cmd) => self.handle_stop_resume(res, target, cmd),
            Command::NoAckMode(cmd) => self.handle_no_ack_mode(res, target, cmd),
            Command::XLowcasePacket(cmd) => self.handle_x_lowcase_packet(res, target, cmd),
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
            Command::FlashOperations(cmd) => self.handle_flash_operations(res, target, cmd),
            Command::HostIo(cmd) => self.handle_host_io(res, target, cmd),
            Command::ExecFile(cmd) => self.handle_exec_file(res, target, cmd),
            Command::Auxv(cmd) => self.handle_auxv(res, target, cmd),
            Command::ThreadExtraInfo(cmd) => self.handle_thread_extra_info(res, target, cmd),
            Command::LldbRegisterInfo(cmd) => self.handle_lldb_register_info(res, target, cmd),
            Command::LibrariesSvr4(cmd) => self.handle_libraries_svr4(res, target, cmd),
            Command::Libraries(cmd) => self.handle_libraries(res, target, cmd),
            Command::Tracepoints(cmd) => self.handle_tracepoints(res, target, cmd),
            // in the worst case, the command could not be parsed...
            Command::Unknown(cmd) => {
                // HACK: if the user accidentally sends a resume command to a
                // target without resume support, inform them of their mistake +
                // return a dummy stop reason.
                if target.base_ops().resume_ops().is_none() && target.use_resume_stub() {
                    let is_resume_pkt = cmd
                        .first()
                        .map(|c| matches!(c, b'c' | b'C' | b's' | b'S'))
                        .unwrap_or(false);

                    if is_resume_pkt {
                        warn!("attempted to resume target without resume support!");

                        // TODO: omit this message if non-stop mode is active
                        {
                            let mut res = ResponseWriter::new(res.as_conn(), target.use_rle());
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

#[derive(Copy, Clone)]
#[repr(transparent)]
struct ProtocolFeatures(u8);

// This bitflag is not part of the protocol - it is an internal implementation
// detail. The alternative would be to use multiple `bool` fields, which wastes
// space in minimal `gdbstub` configurations.
bitflags::bitflags! {
    impl ProtocolFeatures: u8 {
        const NO_ACK_MODE = 1 << 0;
        const MULTIPROCESS = 1 << 1;
    }
}

impl ProtocolFeatures {
    #[inline(always)]
    fn no_ack_mode(&self) -> bool {
        self.contains(ProtocolFeatures::NO_ACK_MODE)
    }

    #[inline(always)]
    fn set_no_ack_mode(&mut self, val: bool) {
        self.set(ProtocolFeatures::NO_ACK_MODE, val)
    }

    #[inline(always)]
    fn multiprocess(&self) -> bool {
        self.contains(ProtocolFeatures::MULTIPROCESS)
    }

    #[inline(always)]
    fn set_multiprocess(&mut self, val: bool) {
        self.set(ProtocolFeatures::MULTIPROCESS, val)
    }
}
