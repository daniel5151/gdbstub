use crate::protocol::{common::decode_hex, Command};
use crate::target::Target;

/// Packet parse error.
#[derive(Debug)]
pub enum PacketParseError {
    ChecksumMismatched,
    EmptyBuf,
    MissingChecksum,
    MalformedChecksum,
    MalformedCommand,
    NotASCII,
    UnexpectedHeader(u8),
}

/// Top-Level GDB packet
pub enum Packet<'a> {
    Ack,
    Nack,
    Interrupt,
    Command(Command<'a>),
}

pub struct PacketBuf<'a> {
    buf: &'a mut [u8],
    body_range: core::ops::Range<usize>,
}

impl<'a> PacketBuf<'a> {
    /// Validate the contents of the raw packet buffer, checking for checksum
    /// consistency, structural correctness, and ASCII validation.
    pub fn new(pkt_buf: &'a mut [u8]) -> Result<PacketBuf<'a>, PacketParseError> {
        // validate the packet is valid ASCII
        if !pkt_buf.is_ascii() {
            return Err(PacketParseError::NotASCII);
        }

        let end_of_body = pkt_buf
            .iter()
            .position(|b| *b == b'#')
            .ok_or(PacketParseError::MissingChecksum)?;

        // split buffer into body and checksum components
        let (body, checksum) = pkt_buf.split_at_mut(end_of_body);
        let body = &mut body[1..]; // skip the '$'
        let checksum = &mut checksum[1..][..2]; // skip the '#'

        // validate the checksum
        let checksum = decode_hex(checksum).map_err(|_| PacketParseError::MalformedChecksum)?;

        if body.iter().fold(0u8, |a, x| a.wrapping_add(*x)) != checksum {
            return Err(PacketParseError::ChecksumMismatched);
        }

        if log_enabled!(log::Level::Trace) {
            // SAFETY: body confirmed to be `is_ascii()`
            let body = unsafe { core::str::from_utf8_unchecked(body) };
            trace!("<-- ${}#{:02x?}", body, checksum);
        }

        Ok(PacketBuf {
            buf: pkt_buf,
            body_range: 1..end_of_body,
        })
    }

    /// (used for tests) Skip the header/checksum trimming stage, but _not_ the
    /// ASCII validation.
    #[cfg(test)]
    pub fn new_with_raw_body(body: &'a mut [u8]) -> Result<PacketBuf<'a>, PacketParseError> {
        // validate the packet is valid ASCII
        if !body.is_ascii() {
            return Err(PacketParseError::NotASCII);
        }

        let len = body.len();
        Ok(PacketBuf {
            buf: body,
            body_range: 0..len,
        })
    }

    pub fn trim_start_body_bytes(self, n: usize) -> Self {
        PacketBuf {
            buf: self.buf,
            body_range: (self.body_range.start + n)..self.body_range.end,
        }
    }

    pub fn as_body(&'a self) -> &'a [u8] {
        &self.buf[self.body_range.clone()]
    }

    /// Return a mut reference to slice of the packet buffer corresponding to
    /// the current body.
    pub fn into_body(self) -> &'a mut [u8] {
        &mut self.buf[self.body_range]
    }

    pub fn into_body_str(self) -> &'a str {
        // SAFETY: buffer confirmed to be `is_ascii()` in `new`, and no other PacketBuf
        // member allow arbitrary modification of `self.buf`.
        unsafe { core::str::from_utf8_unchecked(&self.buf[self.body_range.clone()]) }
    }

    /// Return a mut reference to the _entire_ underlying packet buffer, and the
    /// current body's range.
    #[allow(dead_code)]
    pub fn into_raw_buf(self) -> (&'a mut [u8], core::ops::Range<usize>) {
        (self.buf, self.body_range)
    }
}

impl<'a> Packet<'a> {
    pub fn from_buf(
        target: &mut impl Target,
        buf: &'a mut [u8],
    ) -> Result<Packet<'a>, PacketParseError> {
        // cannot have empty packet
        if buf.is_empty() {
            return Err(PacketParseError::EmptyBuf);
        }

        match buf[0] {
            b'$' => Ok(Packet::Command(
                Command::from_packet(target, PacketBuf::new(buf)?)
                    // TODO?: preserve command parse error context
                    .map_err(|_| PacketParseError::MalformedCommand)?,
            )),
            b'+' => Ok(Packet::Ack),
            b'-' => Ok(Packet::Nack),
            0x03 => Ok(Packet::Interrupt),
            _ => Err(PacketParseError::UnexpectedHeader(buf[0])),
        }
    }
}
