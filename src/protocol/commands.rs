use core::convert::TryFrom;

// TODO: figure out how to make it accept exprs _and_ blocks
// TODO: use a trie structure for more efficient longest-prefix matching
macro_rules! prefix_match {
    (
        match $val:expr => [$name:ident|$rest:ident] {
            $($prefix:literal => $arm:block)*
            _ => $other:block
        }
    ) => {{
        let $name;
        let $rest;
        match $val {
            $(_ if $val.starts_with($prefix) => {
                $name = &$val[..$prefix.len()];
                $rest = &$val[$prefix.len()..];
                $arm
            })*
            _ => $other
        }
    }};
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
            pub fn from_packet_body(body: &'a str) -> Result<Command<'a>, CommandParseError<'a>> {
                if body.is_empty() {
                    // TODO: double check this
                    return Err(CommandParseError::Empty);
                }

                let command = prefix_match! {
                    match body => [name | rest] {
                        $($name => {
                            let cmd = $command::try_from(rest)
                                .map_err(|_| CommandParseError::MalformedCommand(name))?;
                            Command::$command(cmd)
                        })*
                        _ => { Command::Unknown(body) }
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
    "m" => _m::m,
    "M" => _m_upcase::M<'a>,
    "qAttached" => _qAttached::qAttached,
    "qC" => _qC::qC,
    "qfThreadInfo" => _qfThreadInfo::qfThreadInfo,
    "qsThreadInfo" => _qsThreadInfo::qsThreadInfo,
    "qSupported" => _qSupported::qSupported<'a>,
    "qXfer:features:read" => _qXfer_features_read::qXferFeaturesRead<'a>,
    "s" => _s::s,
    "z" => _z::z,
    "Z" => _z_upcase::Z,

    // Order Matters (because of prefix matching)
    "vCont?" => vCont_question_mark::vContQuestionMark,
    "vCont" => _vCont::vCont<'a>,
    "vKill" => _vKill::vKill,
}
