use crate::protocol::packet::PacketBuf;

pub mod prelude {
    pub use super::ParseCommand;
    pub use crate::protocol::common::*;
    pub use crate::protocol::packet::PacketBuf;
}

// TODO: figure out how to make it accept exprs _and_ blocks
// TODO: use a trie structure for more efficient longest-prefix matching
macro_rules! prefix_match {
    (
        match ($val:expr) {
            $($prefix:literal => $arm:block)*
            _ => $other:block
        }
    ) => {{
        #[allow(clippy::string_lit_as_bytes)]
        match $val {
            $(_ if $val.starts_with($prefix.as_bytes()) => {
                $arm
            })*
            _ => $other
        }
    }};
}

pub trait ParseCommand<'a>: Sized {
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self>;
}

macro_rules! commands {
    ($($name:literal => $mod:ident::$command:ident$(<$lifetime:lifetime>)?,)*) => {
        $(
            #[allow(non_snake_case, non_camel_case_types)]
            pub mod $mod;
        )*
        $(pub use $mod::$command;)*

        /// GDB commands
        #[allow(non_camel_case_types)]
        #[derive(Debug)]
        pub enum Command<'a> {
            $($command($command<$($lifetime)?>),)*
            Unknown(&'a str),
        }

        impl<'a> Command<'a> {
            pub fn from_packet(
                buf: PacketBuf<'a>
            ) -> Result<Command<'a>, CommandParseError<'a>> {
                if buf.as_body().is_empty() {
                    return Err(CommandParseError::Empty);
                }

                let command = prefix_match! {
                    match (buf.as_body()) {
                        $($name => {
                            let buf = buf.trim_start_body_bytes($name.len());
                            let cmd = $command::from_packet(buf)
                                .ok_or(CommandParseError::MalformedCommand($name))?;
                            Command::$command(cmd)
                        })*
                        _ => { Command::Unknown(buf.into_body_str()) }
                    }
                };

                Ok(command)
            }
        }

    };
}

/// Command parse error
// TODO: add more granular errors
#[derive(Debug)]
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
    "qAttached" => _qAttached::qAttached,
    "qC" => _qC::qC,
    "qfThreadInfo" => _qfThreadInfo::qfThreadInfo,
    "qRcmd" => _qRcmd::qRcmd<'a>,
    "qsThreadInfo" => _qsThreadInfo::qsThreadInfo,
    "qSupported" => _qSupported::qSupported<'a>,
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
