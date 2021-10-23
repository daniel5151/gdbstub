use paste::paste;

use crate::protocol::packet::PacketBuf;
use crate::target::Target;

pub(self) mod prelude {
    pub use super::ParseCommand;
    pub use crate::common::*;
    pub use crate::protocol::common::hex::{
        decode_bin_buf, decode_hex, decode_hex_buf, is_hex, HexString,
    };
    pub use crate::protocol::common::lists;
    pub use crate::protocol::common::qxfer::{ParseAnnex, QXferReadBase};
    pub use crate::protocol::common::thread_id::{
        IdKind, SpecificIdKind, SpecificThreadId, ThreadId,
    };
    pub use crate::protocol::common::Bstr;
    pub use crate::protocol::packet::PacketBuf;
    pub use crate::util::no_panic_iter::SliceExt;
    pub use core::convert::{TryFrom, TryInto};
}

pub trait ParseCommand<'a>: Sized {
    /// Try to parse a packet from the packet buffer.
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self>;
}

// Breakpoint packets are special-cased, as the "Z" packet is parsed differently
// depending on whether or not the target implements the `Agent` extension.
//
// While it's entirely possible to eagerly parse the "Z" packet for bytecode,
// doing so would unnecessary bloat implementations that do not support
// evaluating agent expressions.

macro_rules! commands {
    (
        $(
            $ext:ident $(use $lt:lifetime)? {
                $($name:literal => $mod:ident::$command:ident$(<$lifetime:lifetime>)?,)*
            }
        )*
    ) => {paste! {
        $($(
            #[allow(non_snake_case, non_camel_case_types)]
            pub mod $mod;
        )*)*
        pub mod breakpoint;

        pub mod ext {
            $(
                #[allow(non_camel_case_types)]
                pub enum [<$ext:camel>] $(<$lt>)? {
                    $($command(super::$mod::$command<$($lifetime)?>),)*
                }
            )*

            use super::breakpoint::{BasicBreakpoint, BytecodeBreakpoint};
            #[allow(non_camel_case_types)]
            pub enum Breakpoints<'a> {
                z(BasicBreakpoint<'a>),
                Z(BasicBreakpoint<'a>),
                ZWithBytecode(BytecodeBreakpoint<'a>),
            }

        }

        /// GDB commands
        pub enum Command<'a> {
            $(
                [<$ext:camel>](ext::[<$ext:camel>]$(<$lt>)?),
            )*
            Breakpoints(ext::Breakpoints<'a>),
            Unknown(&'a [u8]),
        }

        impl<'a> Command<'a> {
            pub fn from_packet(
                target: &mut impl Target,
                mut buf: PacketBuf<'a>
            ) -> Option<Command<'a>> {
                // HACK: this locally-scoped trait enables using identifiers
                // that aren't top-level `Target` IDETs to split-up the packet
                // parsing code.
                trait Hack {
                    fn support_base(&mut self) -> Option<()>;
                    fn support_single_register_access(&mut self) -> Option<()>;
                    fn support_reverse_step(&mut self) -> Option<()>;
                    fn support_reverse_cont(&mut self) -> Option<()>;
                    fn support_x_upcase_packet(&mut self) -> Option<()>;
                }

                impl<T: Target> Hack for T {
                    fn support_base(&mut self) -> Option<()> {
                        Some(())
                    }

                    fn support_single_register_access(&mut self) -> Option<()> {
                        use crate::target::ext::base::BaseOps;
                        match self.base_ops() {
                            BaseOps::SingleThread(ops) => ops.support_single_register_access().map(drop),
                            BaseOps::MultiThread(ops) => ops.support_single_register_access().map(drop),
                        }
                    }

                    fn support_reverse_step(&mut self) -> Option<()> {
                        use crate::target::ext::base::BaseOps;
                        match self.base_ops() {
                            BaseOps::SingleThread(ops) => ops.support_reverse_step().map(drop),
                            BaseOps::MultiThread(ops) => ops.support_reverse_step().map(drop),
                        }
                    }

                    fn support_reverse_cont(&mut self) -> Option<()> {
                        use crate::target::ext::base::BaseOps;
                        match self.base_ops() {
                            BaseOps::SingleThread(ops) => ops.support_reverse_cont().map(drop),
                            BaseOps::MultiThread(ops) => ops.support_reverse_cont().map(drop),
                        }
                    }

                    fn support_x_upcase_packet(&mut self) -> Option<()> {
                        if self.use_x_upcase_packet() {
                            Some(())
                        } else {
                            None
                        }
                    }
                }

                // TODO?: use tries for more efficient longest prefix matching

                $(
                #[allow(clippy::string_lit_as_bytes)]
                if target.[< support_ $ext >]().is_some() {
                    $(
                    if buf.strip_prefix($name.as_bytes()) {
                        crate::__dead_code_marker!($name, "prefix_match");

                        let cmd = $mod::$command::from_packet(buf)?;

                        return Some(
                            Command::[<$ext:camel>](
                                ext::[<$ext:camel>]::$command(cmd)
                            )
                        )
                    }
                    )*
                }
                )*

                if let Some(_breakpoint_ops) = target.support_breakpoints() {
                    use breakpoint::{BasicBreakpoint, BytecodeBreakpoint};

                    if buf.strip_prefix(b"z") {
                        let cmd = BasicBreakpoint::from_slice(buf.into_body())?;
                        return Some(Command::Breakpoints(ext::Breakpoints::z(cmd)))
                    }

                    if buf.strip_prefix(b"Z") {
                        // TODO: agent bytecode currently unimplemented
                        if true {
                            let cmd = BasicBreakpoint::from_slice(buf.into_body())?;
                            return Some(Command::Breakpoints(ext::Breakpoints::Z(cmd)))
                        } else {
                            let cmd = BytecodeBreakpoint::from_slice(buf.into_body())?;
                            return Some(Command::Breakpoints(ext::Breakpoints::ZWithBytecode(cmd)))
                        }
                    }
                }

                Some(Command::Unknown(buf.into_body()))
            }
        }
    }};
}

commands! {
    base use 'a {
        "?" => question_mark::QuestionMark,
        "c" => _c::c<'a>,
        "D" => _d_upcase::D,
        "g" => _g::g,
        "G" => _g_upcase::G<'a>,
        "H" => _h_upcase::H,
        "k" => _k::k,
        "m" => _m::m<'a>,
        "M" => _m_upcase::M<'a>,
        "qAttached" => _qAttached::qAttached,
        "qfThreadInfo" => _qfThreadInfo::qfThreadInfo,
        "QStartNoAckMode" => _QStartNoAckMode::QStartNoAckMode,
        "qsThreadInfo" => _qsThreadInfo::qsThreadInfo,
        "qSupported" => _qSupported::qSupported<'a>,
        "qXfer:features:read" => _qXfer_features_read::qXferFeaturesRead<'a>,
        "s" => _s::s<'a>,
        "T" => _t_upcase::T,
        "vCont" => _vCont::vCont<'a>,
        "vKill" => _vKill::vKill,
    }

    x_upcase_packet use 'a {
        "X" => _x_upcase::X<'a>,
    }

    single_register_access use 'a {
        "p" => _p::p<'a>,
        "P" => _p_upcase::P<'a>,
    }

    extended_mode use 'a {
        "!" => exclamation_mark::ExclamationMark,
        "QDisableRandomization" => _QDisableRandomization::QDisableRandomization,
        "QEnvironmentHexEncoded" => _QEnvironmentHexEncoded::QEnvironmentHexEncoded<'a>,
        "QEnvironmentReset" => _QEnvironmentReset::QEnvironmentReset,
        "QEnvironmentUnset" => _QEnvironmentUnset::QEnvironmentUnset<'a>,
        "QSetWorkingDir" => _QSetWorkingDir::QSetWorkingDir<'a>,
        "QStartupWithShell" => _QStartupWithShell::QStartupWithShell,
        "R" => _r_upcase::R,
        "vAttach" => _vAttach::vAttach,
        "vRun" => _vRun::vRun<'a>,
    }

    monitor_cmd use 'a {
        "qRcmd" => _qRcmd::qRcmd<'a>,
    }

    section_offsets {
        "qOffsets" => _qOffsets::qOffsets,
    }

    reverse_cont {
        "bc" => _bc::bc,
    }

    reverse_step {
        "bs" => _bs::bs,
    }

    memory_map use 'a {
        "qXfer:memory-map:read" => _qXfer_memory_map::qXferMemoryMapRead<'a>,
    }

    auxv use 'a {
        "qXfer:auxv:read" => _qXfer_auxv_read::qXferAuxvRead<'a>,
    }

    exec_file use 'a {
        "qXfer:exec-file:read" => _qXfer_exec_file::qXferExecFileRead<'a>,
    }

    host_io use 'a {
        "vFile:open" => _vFile_open::vFileOpen<'a>,
        "vFile:close" => _vFile_close::vFileClose,
        "vFile:pread" => _vFile_pread::vFilePread<'a>,
        "vFile:pwrite" => _vFile_pwrite::vFilePwrite<'a>,
        "vFile:fstat" => _vFile_fstat::vFileFstat,
        "vFile:unlink" => _vFile_unlink::vFileUnlink<'a>,
        "vFile:readlink" => _vFile_readlink::vFileReadlink<'a>,
        "vFile:setfs" => _vFile_setfs::vFileSetfs,
    }

    catch_syscalls use 'a {
        "QCatchSyscalls" => _QCatchSyscalls::QCatchSyscalls<'a>,
    }
}
