use log::*;

pub mod q_supported;
pub use q_supported::QSupported;

#[derive(PartialEq, Eq, Debug)]
pub enum Command<'a> {
    Ack,
    QSupported(QSupported<'a>),
    Retransmit,
    Unknown,
}

#[derive(Debug)]
pub enum Error<'a> {
    EmptyBuf,
    MalformedChecksum,
    MalformedCommand(&'a str),
    MismatchedChecksum,
    NotASCII,
    UnexpectedHeader(u8),
}

impl<'a> Command<'a> {
    // TODO: massively improve parsing errors
    pub fn from_packet(buf: &'a [u8]) -> Result<Command<'a>, Error<'a>> {
        if buf.is_empty() {
            // cannot have empty packet
            return Err(Error::EmptyBuf);
        }

        match buf[0] {
            b'$' => { /* continue on to parse body */ }
            b'+' => return Ok(Command::Ack),
            b'-' => return Ok(Command::Retransmit),
            _ => return Err(Error::UnexpectedHeader(buf[0])),
        }

        let mut buf = buf[1..].split(|b| *b == b'#');
        let body = buf.next().unwrap();
        let checksum = buf.next().unwrap();

        let checksum = core::str::from_utf8(checksum).map_err(|_| Error::NotASCII)?;
        if !checksum.is_ascii() {
            return Err(Error::NotASCII);
        }
        let checksum = u8::from_str_radix(checksum, 16).map_err(|_| Error::MalformedChecksum)?;

        trace!("${}#{:02x?}", String::from_utf8_lossy(&body), checksum);

        if body.iter().sum::<u8>() != checksum {
            return Err(Error::MismatchedChecksum);
        }

        let body = core::str::from_utf8(&body).map_err(|_| Error::NotASCII)?;
        if !body.is_ascii() {
            return Err(Error::NotASCII);
        }

        Ok(Command::from_body(body)?)
    }

    pub fn from_body(body: &'a str) -> Result<Command<'a>, Error> {
        if body.is_empty() {
            // TODO: double check this
            return Ok(Command::Unknown);
        }

        let mut body = body.split(':');
        let cmd_name = body.next().unwrap();
        let cmd_body = body.next(); // optional argument

        let command = match cmd_name {
            "qSupported" => Command::QSupported(
                QSupported::from_cmd_body(cmd_body)
                    .map_err(|_| Error::MalformedCommand(cmd_name))?,
            ),
            _ => Command::Unknown,
        };

        Ok(command)
    }
}
