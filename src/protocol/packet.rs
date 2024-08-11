use crate::protocol::commands::Command;
use crate::protocol::common::hex::decode_hex;
use crate::target::Target;

/// Packet parse error.
#[derive(Debug)]
pub enum PacketParseError {
    #[allow(dead_code)] // used as part of Debug impl
    ChecksumMismatched {
        checksum: u8,
        calculated: u8,
    },
    EmptyBuf,
    MissingChecksum,
    MalformedChecksum,
    MalformedCommand,
    #[allow(dead_code)] // used as part of Debug impl
    UnexpectedHeader(u8),
}

/// Top-Level GDB packet
pub enum Packet<'a> {
    Ack,
    Nack,
    Interrupt,
    Command(Command<'a>),
}

/// Wrapper around a byte buffer containing a GDB packet, while also tracking
/// the range of the buffer containing the packet's "body".
///
/// A newly constructed `PacketBuf` will have a body that spans the entire data
/// portion of the packet (i.e: `b"$data#checksum"`), but this range can be
/// further restricted as part of packet parsing.
///
/// Notably, `PacketBuf` will _always_ maintain a mutable reference back to the
/// _entire_ underlying packet buffer. This makes it possible to re-use any
/// unused buffer space as "scratch" space. One notable example of this use-case
/// is the 'm' packet, which recycles unused packet buffer space as a buffer for
/// the target's `read_memory` method.
pub struct PacketBuf<'a> {
    buf: &'a mut [u8],
    body_range: core::ops::Range<usize>,
}

impl<'a> PacketBuf<'a> {
    /// Validate the contents of the raw packet buffer, checking for checksum
    /// consistency and structural correctness.
    pub fn new(pkt_buf: &'a mut [u8]) -> Result<PacketBuf<'a>, PacketParseError> {
        if pkt_buf.is_empty() {
            return Err(PacketParseError::EmptyBuf);
        }

        // split buffer into body and checksum components
        let mut parts = pkt_buf[1..].split(|b| *b == b'#');

        let body = parts.next().unwrap(); // spit iter always returns at least one element
        let checksum = parts
            .next()
            .ok_or(PacketParseError::MissingChecksum)?
            .get(..2)
            .ok_or(PacketParseError::MalformedChecksum)?;

        // validate the checksum
        let checksum = decode_hex(checksum).map_err(|_| PacketParseError::MalformedChecksum)?;
        let calculated = body.iter().fold(0u8, |a, x| a.wrapping_add(*x));
        if calculated != checksum {
            return Err(PacketParseError::ChecksumMismatched {
                checksum,
                calculated,
            });
        }

        let body_range = 1..(body.len() + 1); // compensate for the leading '$'

        Ok(PacketBuf {
            buf: pkt_buf,
            body_range,
        })
    }

    /// (used for tests) Create a packet buffer from a raw body buffer, skipping
    /// the header/checksum trimming stage.
    #[cfg(test)]
    pub fn new_with_raw_body(body: &'a mut [u8]) -> Result<PacketBuf<'a>, PacketParseError> {
        let len = body.len();
        Ok(PacketBuf {
            buf: body,
            body_range: 0..len,
        })
    }

    /// Strip the specified prefix from the packet buffer, returning `true` if
    /// there was a prefix match.
    pub fn strip_prefix(&mut self, prefix: &[u8]) -> bool {
        let body = {
            // SAFETY: The public interface of `PacketBuf` ensures that `self.body_range`
            // always stays within the bounds of the provided buffer.
            #[cfg(not(feature = "paranoid_unsafe"))]
            unsafe {
                self.buf.get_unchecked_mut(self.body_range.clone())
            }

            #[cfg(feature = "paranoid_unsafe")]
            &mut self.buf[self.body_range.clone()]
        };

        if body.starts_with(prefix) {
            // SAFETY: if the current buffer range `starts_with` the specified prefix, then
            // it is safe to bump `body_range.start` by the prefix length.
            self.body_range = (self.body_range.start + prefix.len())..self.body_range.end;
            true
        } else {
            false
        }
    }

    /// Return a mutable reference to slice of the packet buffer corresponding
    /// to the current body.
    pub fn into_body(self) -> &'a mut [u8] {
        // SAFETY: The public interface of `PacketBuf` ensures that `self.body_range`
        // always stays within the bounds of the provided buffer.
        #[cfg(not(feature = "paranoid_unsafe"))]
        unsafe {
            self.buf.get_unchecked_mut(self.body_range)
        }

        #[cfg(feature = "paranoid_unsafe")]
        &mut self.buf[self.body_range]
    }

    /// Return a mutable reference to the _entire_ underlying packet buffer, and
    /// the current body's range.
    pub fn into_raw_buf(self) -> (&'a mut [u8], core::ops::Range<usize>) {
        (self.buf, self.body_range)
    }

    /// Returns the length of the _entire_ underlying packet buffer - not just
    /// the length of the current range.
    ///
    /// This method is used when handing the `qSupported` packet in order to
    /// obtain the maximum packet size the stub supports.
    pub fn full_len(&self) -> usize {
        self.buf.len()
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
                    .ok_or(PacketParseError::MalformedCommand)?,
            )),
            b'+' => Ok(Packet::Ack),
            b'-' => Ok(Packet::Nack),
            0x03 => Ok(Packet::Interrupt),
            _ => Err(PacketParseError::UnexpectedHeader(buf[0])),
        }
    }
}
