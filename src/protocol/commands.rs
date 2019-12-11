// TODO: figure out how to make it accept exprs _and_ blocks
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
        #[derive(PartialEq, Eq, Debug)]
        pub enum Command<'a> {
            $($command($command<$($lifetime)?>),)*
            Unknown,
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
                            let cmd = $command::parse(rest)
                                .map_err(|_| CommandParseError::MalformedCommand(name))?;
                            Command::$command(cmd)
                        })*
                        _ => { Command::Unknown }
                    }
                };

                Ok(command)
            }
        }

    };
}

/// Command parse error
#[derive(Debug)]
pub enum CommandParseError<'a> {
    Empty,
    /// catch-all
    MalformedCommand(&'a str),
}

commands! {
    "?" => question_mark::QuestionMark,
    "D" => _D::D,
    "g" => _g::g,
    "H" => _H::H,
    "m" => _m::m,
    "qAttached" => _qAttached::qAttached,
    "qC" => _qC::qC,
    "qfThreadInfo" => _qfThreadInfo::qfThreadInfo,
    "qsThreadInfo" => _qsThreadInfo::qsThreadInfo,
    "qSupported" => _qSupported::qSupported<'a>,
    "qXfer:features:read" => _qXfer_features_read::qXferFeaturesRead<'a>,
}
