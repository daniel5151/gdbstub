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
mod lldb_error_strings;
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
            let err = match self {
                Ok(v) => return Ok(v),
                Err(TargetError::Fatal(e)) => return Err(InternalError::TargetError(e)),
                // Recoverable errors:
                // Error code 121 corresponds to `EREMOTEIO` lol
                Err(TargetError::NonFatal) => InternalError::NonFatalError(121),
                Err(TargetError::Errno(code)) => InternalError::NonFatalError(code),
                Err(TargetError::NonFatalMsg(code, msg)) => InternalError::NonFatalErrorMsg(code, msg),
                #[cfg(feature = "alloc")]
                Err(TargetError::NonFatalMsgAlloc(code, msg)) => {
                    InternalError::NonFatalErrorMsgAlloc(code, msg)
                }
                #[cfg(feature = "std")]
                Err(TargetError::Io(e)) => {
                    InternalError::NonFatalError(e.raw_os_error().unwrap_or(121) as u8)
                }
            };

            Err(err)
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
                    Err(err @ InternalError::NonFatalErrorMsg(_, _)) => {
                        self.handle_non_fatal_error_msg(&mut res, target, err)?
                    }
                    #[cfg(feature = "alloc")]
                    Err(err @ InternalError::NonFatalErrorMsgAlloc(_, _)) => {
                        self.handle_non_fatal_error_msg(&mut res, target, err)?
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
            Command::LldbErrorStrings(cmd) => self.handle_lldb_error_strings(res, target, cmd),
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

    fn handle_non_fatal_error_msg(
        &mut self,
        res: &mut ResponseWriter<'_, C>,
        target: &mut T,
        err: InternalError<T::Error, C::Error>,
    ) -> Result<Option<DisconnectReason>, InternalError<T::Error, C::Error>> {
        use InternalError::*;

        let (code, msg) = match err {
            NonFatalErrorMsg(code, msg) => (code, msg),
            #[cfg(feature = "alloc")]
            NonFatalErrorMsgAlloc(code, ref msg) => (code, msg.as_ref()),
            _ => unreachable!(),
        };

        if target.use_error_messages() {
            return self.handle_non_fatal_error_msg_impl(res, code, msg);
        }

        // Fallback: EXX
        res.write_str("E")?;
        res.write_num(code)?;
        Ok(None)
    }

    #[inline(never)]
    fn handle_non_fatal_error_msg_impl(
        &mut self,
        res: &mut ResponseWriter<'_, C>,
        code: u8,
        msg: &str,
    ) -> Result<Option<DisconnectReason>, InternalError<T::Error, C::Error>> {
        if self.features.gdb_error_message() {
            // GDB native: E,errtext
            //
            // The GDB spec (as of June 2024) forbids '$' and '#' in the
            // error message, as they are used as packet delimiters.
            res.write_str("E,")?;
            let mut has_reserved = false;
            for c in msg.chars() {
                match c {
                    '$' => {
                        res.write_str("(reserved char $)")?;
                        has_reserved = true;
                    }
                    '#' => {
                        res.write_str("(reserved char #)")?;
                        has_reserved = true;
                    }
                    c => {
                        let mut b = [0; 4];
                        res.write_str(c.encode_utf8(&mut b))?;
                    }
                }
            }
            if has_reserved {
                res.write_str("\n[gdbstub]: error messages cannot contain '$' or '#'")?;
            }
            return Ok(None);
        } else if self.features.lldb_error_strings() {
            // LLDB: EXX;errtext (hex)
            res.write_str("E")?;
            res.write_num(code)?;
            res.write_str(";")?;
            res.write_hex_buf(msg.as_bytes())?;
            return Ok(None);
        }

        // Fallback: EXX
        res.write_str("E")?;
        res.write_num(code)?;
        Ok(None)
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
        const GDB_ERROR_MESSAGE = 1 << 2;
        const LLDB_ERROR_STRINGS = 1 << 3;
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

    #[inline(always)]
    fn gdb_error_message(&self) -> bool {
        self.contains(ProtocolFeatures::GDB_ERROR_MESSAGE)
    }

    #[inline(always)]
    fn set_gdb_error_message(&mut self, val: bool) {
        self.set(ProtocolFeatures::GDB_ERROR_MESSAGE, val)
    }

    #[inline(always)]
    fn lldb_error_strings(&self) -> bool {
        self.contains(ProtocolFeatures::LLDB_ERROR_STRINGS)
    }

    #[inline(always)]
    fn set_lldb_error_strings(&mut self, val: bool) {
        self.set(ProtocolFeatures::LLDB_ERROR_STRINGS, val)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conn::Connection;
    use crate::protocol::ResponseWriter;
    use crate::target::Target;
    use alloc::vec::Vec;

    struct MockConnection(Vec<u8>);

    impl Connection for MockConnection {
        type Error = ();

        fn write(&mut self, byte: u8) -> Result<(), Self::Error> {
            self.0.push(byte);
            Ok(())
        }

        fn flush(&mut self) -> Result<(), Self::Error> {
            Ok(())
        }
    }

    #[derive(Debug, Default, Clone, PartialEq)]
    struct MockRegisters;

    impl crate::arch::Registers for MockRegisters {
        type ProgramCounter = u32;
        fn pc(&self) -> u32 {
            0
        }
        fn gdb_serialize(&self, _: impl FnMut(Option<u8>)) {}
        fn gdb_deserialize(&mut self, _: &[u8]) -> Result<(), ()> {
            Ok(())
        }
    }

    struct MockArch;

    impl crate::arch::Arch for MockArch {
        type Usize = u32;
        type Registers = MockRegisters;
        type BreakpointKind = ();
        type RegId = ();
    }

    struct MockTarget;

    impl Target for MockTarget {
        type Arch = MockArch;
        type Error = ();

        fn base_ops(&mut self) -> crate::target::ext::base::BaseOps<'_, Self::Arch, Self::Error> {
            unimplemented!()
        }
    }

    fn assert_packet(data: &[u8], body: &[u8]) {
        assert_eq!(data[0], b'$');
        let hash_pos = data.iter().rposition(|&b| b == b'#').unwrap();
        assert_eq!(
            core::str::from_utf8(&data[1..hash_pos]).unwrap(),
            core::str::from_utf8(body).unwrap()
        );
        // checksum verification
        let sum: u8 = body.iter().fold(0, |a, &b| a.wrapping_add(b));
        let hex_sum = &data[hash_pos + 1..];
        let mut expected_hex = [0u8; 2];
        let hi = sum >> 4;
        let lo = sum & 0xf;
        expected_hex[0] = if hi < 10 { b'0' + hi } else { b'a' + hi - 10 };
        expected_hex[1] = if lo < 10 { b'0' + lo } else { b'a' + lo - 10 };
        assert_eq!(hex_sum, &expected_hex);
    }

    #[test]
    fn test_error_packet_generation() {
        let mut target = MockTarget;
        let mut conn = MockConnection(Vec::new());
        let mut stub = GdbStubImpl::<MockTarget, MockConnection>::new();

        // Case 1: Standard GDB (no error-message)
        {
            conn.0.clear();
            let mut res = ResponseWriter::new(&mut conn, false);
            stub.handle_non_fatal_error_msg(
                &mut res,
                &mut target,
                InternalError::NonFatalErrorMsg(0x10, "test error"),
            )
            .unwrap();
            res.flush().unwrap();
            assert_packet(&conn.0, b"E10");
        }

        // Case 2: GDB with error-message
        {
            stub.features.set_gdb_error_message(true);
            conn.0.clear();
            let mut res = ResponseWriter::new(&mut conn, false);
            stub.handle_non_fatal_error_msg(
                &mut res,
                &mut target,
                InternalError::NonFatalErrorMsg(0x10, "test error"),
            )
            .unwrap();
            res.flush().unwrap();
            assert_packet(&conn.0, b"E,test error");
        }

        // Case 3: LLDB with error-strings
        {
            stub.features.set_gdb_error_message(false);
            stub.features.set_lldb_error_strings(true);
            conn.0.clear();
            let mut res = ResponseWriter::new(&mut conn, false);
            stub.handle_non_fatal_error_msg(
                &mut res,
                &mut target,
                InternalError::NonFatalErrorMsg(0x10, "test error"),
            )
            .unwrap();
            res.flush().unwrap();
            assert_packet(&conn.0, b"E10;74657374206572726f72");
        }

        // Case 4: GDB with error-message, but message contains forbidden characters
        {
            stub.features.set_gdb_error_message(true);
            conn.0.clear();
            let mut res = ResponseWriter::new(&mut conn, false);
            stub.handle_non_fatal_error_msg(
                &mut res,
                &mut target,
                InternalError::NonFatalErrorMsg(0x10, "error with $ and #"),
            )
            .unwrap();
            res.flush().unwrap();
            assert_packet(
                &conn.0,
                b"E,error with (reserved char $) and (reserved char #)\n[gdbstub]: error messages cannot contain '$' or '#'",
            );
        }

        // Case 5: Target disables error messages
        {
            struct NoMsgTarget;
            impl Target for NoMsgTarget {
                type Arch = MockArch;
                type Error = ();
                fn base_ops(
                    &mut self,
                ) -> crate::target::ext::base::BaseOps<'_, Self::Arch, Self::Error> {
                    unimplemented!()
                }
                fn use_error_messages(&self) -> bool {
                    false
                }
            }

            let mut target = NoMsgTarget;
            let mut stub = GdbStubImpl::<NoMsgTarget, MockConnection>::new();
            stub.features.set_gdb_error_message(true);
            conn.0.clear();
            let mut res = ResponseWriter::new(&mut conn, false);
            stub.handle_non_fatal_error_msg(
                &mut res,
                &mut target,
                InternalError::NonFatalErrorMsg(0x10, "test error"),
            )
            .unwrap();
            res.flush().unwrap();
            // Should fall back to EXX
            assert_packet(&conn.0, b"E10");
        }
    }
    #[test]
    fn test_qsupported_gating() {
        use crate::protocol::commands::ParseCommand;
        use crate::protocol::commands::prelude::PacketBuf;

        let mut _target = MockTarget;
        let _stub = GdbStubImpl::<MockTarget, MockConnection>::new();

        // Case 1: use_error_messages = true
        {
            let mut buf = [0u8; 128];
            let msg = b":error-message+;multiprocess+";
            buf[..msg.len()].copy_from_slice(msg);
            let packet_buf = PacketBuf::new_with_raw_body(&mut buf[..msg.len()]).unwrap();
            let cmd = crate::protocol::commands::_qSupported::qSupported::from_packet(packet_buf).unwrap();

            let mut features = ProtocolFeatures(0);
            for feature in cmd.features.into_iter(true) {
                let (feature, supported) = feature.unwrap().unwrap();
                match feature {
                    crate::protocol::commands::_qSupported::Feature::ErrorMessage => {
                        features.set_gdb_error_message(supported)
                    }
                    crate::protocol::commands::_qSupported::Feature::Multiprocess => {
                        features.set_multiprocess(supported)
                    }
                }
            }
            assert!(features.gdb_error_message());
            assert!(features.multiprocess());
        }

        // Case 2: use_error_messages = false
        {
            let mut buf = [0u8; 128];
            let msg = b":error-message+;multiprocess+";
            buf[..msg.len()].copy_from_slice(msg);
            let packet_buf = PacketBuf::new_with_raw_body(&mut buf[..msg.len()]).unwrap();
            let cmd = crate::protocol::commands::_qSupported::qSupported::from_packet(packet_buf).unwrap();

            let mut features = ProtocolFeatures(0);
            for feature in cmd.features.into_iter(false) {
                if let Ok(Some((feature, supported))) = feature {
                    match feature {
                        crate::protocol::commands::_qSupported::Feature::ErrorMessage => {
                            features.set_gdb_error_message(supported)
                        }
                        crate::protocol::commands::_qSupported::Feature::Multiprocess => {
                            features.set_multiprocess(supported)
                        }
                    }
                }
            }
            assert!(!features.gdb_error_message());
            assert!(features.multiprocess());
        }
    }
}
