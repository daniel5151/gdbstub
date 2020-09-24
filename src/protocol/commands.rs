use paste::paste;

use crate::protocol::packet::PacketBuf;
use crate::target::Target;

pub(self) mod prelude {
    pub use super::ParseCommand;
    pub use crate::common::*;
    pub use crate::protocol::common::*;
    pub use crate::protocol::packet::PacketBuf;
}

pub trait ParseCommand<'a>: Sized {
    /// Try to parse a packet from the packet buffer.
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self>;
}

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

        pub mod ext {
            $(
                #[allow(non_camel_case_types)]
                pub enum [<$ext:camel>] $(<$lt>)? {
                    $($command(super::$mod::$command<$($lifetime)?>),)*
                }
            )*
        }

        /// GDB commands
        pub enum Command<'a> {
            $(
                [<$ext:camel>](ext::[<$ext:camel>]$(<$lt>)?),
            )*
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

                // This scoped extension trait enables using `base` as an
                // `$ext`, even through the `base` method on `Target` doesn't
                // return an Option.
                trait Hack { fn base(&mut self) -> Option<()> { Some(()) } }
                impl<T: Target> Hack for T {}

                $(
                if target.$ext().is_some() {
                    // TODO?: use tries for more efficient longest prefix matching
                    #[allow(clippy::string_lit_as_bytes)]
                    match body {
                        $(_ if body.starts_with($name.as_bytes()) => {
                            crate::__dead_code_marker!($name, "prefix_match");

                            let buf = buf.trim_start_body_bytes($name.len());
                            let cmd = $mod::$command::from_packet(buf)
                                .ok_or(CommandParseError::MalformedCommand($name))?;

                            return Ok(
                                Command::[<$ext:camel>](
                                    ext::[<$ext:camel>]::$command(cmd)
                                )
                            )
                        })*
                        _ => {},
                    }
                }
                )*

                Ok(Command::Unknown(buf.into_body_str()))
            }
        }
    }};
}

/// Command parse error
// TODO?: add more granular errors to command parsing code
pub enum CommandParseError<'a> {
    Empty,
    /// catch-all
    MalformedCommand(&'a str),
}

commands! {
    base use 'a {
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
        "QStartNoAckMode" => _QStartNoAckMode::QStartNoAckMode,
        "qsThreadInfo" => _qsThreadInfo::qsThreadInfo,
        "qSupported" => _qSupported::qSupported<'a>,
        "qXfer:features:read" => _qXfer_features_read::qXferFeaturesRead<'a>,
        "s" => _s::s,
        "T" => _t_upcase::T,
        "vCont" => _vCont::vCont<'a>,
        "vKill" => _vKill::vKill,
        "z" => _z::z,
        "Z" => _z_upcase::Z,
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
}
