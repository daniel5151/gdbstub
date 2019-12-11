use log::*;

mod commands;
mod response_writer;

pub use commands::*;
pub use response_writer::{Error as ResponseWriterError, ResponseWriter};

/// Packet parse error.
#[derive(Debug)]
pub enum PacketParseError<'a> {
    EmptyBuf,
    MalformedChecksum,
    MalformedCommand(CommandParseError<'a>),
    MismatchedChecksum,
    NotASCII,
    UnexpectedHeader(u8),
}

/// Top-Level GDB packet
#[derive(PartialEq, Eq, Debug)]
pub enum Packet<'a> {
    Ack,
    Nack,
    Command(Command<'a>),
}

impl<'a> Packet<'a> {
    pub fn from_buf(buf: &'a [u8]) -> Result<Packet<'a>, PacketParseError<'a>> {
        // cannot have empty packet
        if buf.is_empty() {
            return Err(PacketParseError::EmptyBuf);
        }

        match buf[0] {
            b'$' => {
                // split buffer into body and checksum components
                let mut buf = buf[1..].split(|b| *b == b'#');
                let body = buf.next().unwrap();
                let checksum = buf.next().unwrap();

                // validate the checksum
                let checksum =
                    core::str::from_utf8(checksum).map_err(|_| PacketParseError::NotASCII)?;
                let checksum = u8::from_str_radix(checksum, 16)
                    .map_err(|_| PacketParseError::MalformedChecksum)?;
                if body.iter().sum::<u8>() != checksum {
                    return Err(PacketParseError::MismatchedChecksum);
                }

                // validate the body is ASCII
                let body = core::str::from_utf8(&body).map_err(|_| PacketParseError::NotASCII)?;
                if !body.is_ascii() {
                    return Err(PacketParseError::NotASCII);
                }

                trace!("<-- ${}#{:02x?}", body, checksum);

                Ok(Packet::Command(
                    Command::from_packet_body(body).map_err(PacketParseError::MalformedCommand)?,
                ))
            }
            b'+' => Ok(Packet::Ack),
            b'-' => Ok(Packet::Nack),
            _ => Err(PacketParseError::UnexpectedHeader(buf[0])),
        }
    }
}
