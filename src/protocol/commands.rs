use crate::protocol::packet::PacketBuf;
use crate::target::Target;

pub(crate) mod prelude {
    pub(crate) use super::ParseCommand;
    pub(crate) use crate::protocol::common::*;
    pub(crate) use crate::protocol::packet::PacketBuf;
    pub(crate) use crate::target::Target;
}

pub trait ParseCommand<'a>: Sized {
    /// Return a boolean indicating if the target implements the protocol
    /// extension related to this packet.
    ///
    /// A typical implementation should look like
    /// `target.<feature>().is_some()`.
    ///
    /// The default implementation will simply return `true`, implying that the
    /// packet is _always_ required (i.e: is part of the base protocol).
    #[inline(always)]
    fn __protocol_hint(target: &mut impl Target) -> bool {
        let _ = target;
        true
    }

    /// Helper method to call `ParseCommand::__protocol_hint` on directly on a
    /// `packet` reference.
    #[inline(always)]
    fn __protocol_hint_(&self, target: &mut impl Target) -> bool {
        Self::__protocol_hint(target)
    }

    /// Try to parse a packet from the packet buffer.
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self>;
}

macro_rules! commands {
    (
        $($name:literal => $mod:ident::$command:ident$(<$lifetime:lifetime>)?,)*
    ) => {
        $(
            #[allow(non_snake_case, non_camel_case_types)]
            pub mod $mod;
        )*
        $(pub use $mod::$command;)*

        /// GDB commands
        #[allow(non_camel_case_types)]
        pub enum Command<'a> {
            $($command($command<$($lifetime)?>),)*
            Unknown(&'a str),
        }

        impl<'a> Command<'a> {
            pub fn from_packet(
                target: &mut impl Target,
                buf: PacketBuf<'a>
            ) -> Result<Command<'a>, CommandParseError<'a>> {
                if buf.as_body().is_empty() {
                    return Err(CommandParseError::Empty);
                }

                let body = buf.as_body();

                // TODO?: use a trie for more efficient longest prefix matching
                #[allow(clippy::string_lit_as_bytes)]
                let command = match body {
                    $(
                        // (see the comment in `mod gdbstub_impl` for info on __protocol_hint_)
                        _ if <$command<$($lifetime)?>>::__protocol_hint(target)
                         && body.starts_with($name.as_bytes()) =>
                        {
                            crate::__dead_code_marker!($name, "prefix_match");

                            let buf = buf.trim_start_body_bytes($name.len());
                            let cmd = $command::from_packet(buf)
                                .ok_or(CommandParseError::MalformedCommand($name))?;
                            Command::$command(cmd)
                        }
                    )*
                    _ => Command::Unknown(buf.into_body_str()),
                };

                Ok(command)
            }
        }

    };
}

/// Command parse error
// TODO?: add more granular errors to command parsing code
pub enum CommandParseError<'a> {
    Empty,
    /// catch-all
    MalformedCommand(&'a str),
}

commands! {
    "?" => question_mark::QuestionMark,
    "c" => _c::c,
    "D" => _d_upcase::D,
    "g" => _g::g,
    "G" => _g_upcase::G<'a>,
    "H" => _h_upcase::H,
    "k" => _k::k,
    "m" => _m::m<'a>,
    "M" => _m_upcase::M<'a>,
    "p" => _p::p,
    "P" => _p_upcase::P<'a>,
    "qAttached" => _qAttached::qAttached,
    "qfThreadInfo" => _qfThreadInfo::qfThreadInfo,
    "qRcmd" => _qRcmd::qRcmd<'a>,
    "QStartNoAckMode" => _QStartNoAckMode::QStartNoAckMode,
    "qsThreadInfo" => _qsThreadInfo::qsThreadInfo,
    "qSupported" => _qSupported::qSupported<'a>,
    "qOffsets" => _qOffsets::qOffsets,
    "qXfer:features:read" => _qXfer_features_read::qXferFeaturesRead<'a>,
    "s" => _s::s,
    "T" => _t_upcase::T,
    "z" => _z::z,
    "Z" => _z_upcase::Z,

    // Order Matters (because of prefix matching)
    "vCont?" => vCont_question_mark::vContQuestionMark,
    "vCont" => _vCont::vCont<'a>,
    "vKill" => _vKill::vKill,
}
