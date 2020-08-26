use btoi::{btou_radix, ParseIntegerError};
use num_traits::{CheckedAdd, CheckedMul, FromPrimitive, Zero};

#[inline]
pub fn decode_hex<I>(buf: &[u8]) -> Result<I, ParseIntegerError>
where
    I: FromPrimitive + Zero + CheckedAdd + CheckedMul,
{
    btou_radix(buf, 16)
}

pub enum DecodeHexBufError {
    NotAscii,
    NotEvenLen,
}

fn ascii2byte(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        b'x' | b'X' => Some(0),
        _ => None,
    }
}

/// Decode a GDB hex string into a byte slice _in place_.
///
/// GDB hex strings may include "xx", which represent "missing" data. This
/// method simply treats "xx" as 0x00.
// TODO: maybe don't blindly translate "xx" as 0x00?
pub fn decode_hex_buf(buf: &mut [u8]) -> Result<&mut [u8], DecodeHexBufError> {
    use DecodeHexBufError::*;

    if buf.len() % 2 != 0 {
        return Err(NotEvenLen);
    }

    let decoded_len = buf.len() / 2;
    for i in 0..decoded_len {
        let b = ascii2byte(buf[i * 2]).ok_or(NotAscii)? << 4
            | ascii2byte(buf[i * 2 + 1]).ok_or(NotAscii)?;
        buf[i] = b as u8;
    }

    Ok(&mut buf[..decoded_len])
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum EncodeHexBufError {
    SmallBuffer,
}

/// Encode a GDB hex string into a byte slice _in place_.
///
/// The data to be encoded should be copied into the buffer from
/// `buf[start_idx..]`. The buffer itself must be at least `data.len() * 2`
/// bytes in size, as each byte is expanded into a two byte hex string.
#[allow(dead_code)]
pub fn encode_hex_buf(buf: &mut [u8], start_idx: usize) -> Result<&mut [u8], EncodeHexBufError> {
    use EncodeHexBufError::*;

    let len = buf.len() - start_idx;
    let encoded_len = len * 2;

    if buf.len() < encoded_len {
        return Err(SmallBuffer);
    }

    for i in 0..encoded_len {
        let byte = buf[start_idx + i / 2];
        let nybble = if i % 2 == 0 {
            // high
            (byte & 0xf0) >> 4
        } else {
            // low
            byte & 0x0f
        };

        buf[i] = match nybble {
            0x0..=0x9 => b'0' + nybble,
            0xa..=0xf => b'A' + (nybble - 0xa),
            _ => unreachable!(), // could be unreachable_unchecked...
        };
    }

    Ok(&mut buf[..encoded_len])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_hex_simple() {
        let payload = [0xde, 0xad, 0xbe, 0xef];
        let mut buf = [0; 16];

        let start_idx = buf.len() - payload.len();

        // copy the payload into the buffer
        buf[start_idx..].copy_from_slice(&payload);
        let out = encode_hex_buf(&mut buf, start_idx).unwrap();

        assert_eq!(out, b"DEADBEEF");
    }

    #[test]
    fn encode_hex_in_chunks() {
        let payload = (0..=255).collect::<Vec<u8>>();
        let mut out = Vec::new();

        let mut buf = [0; 30];

        for c in payload.chunks(15) {
            let start_idx = buf.len() - c.len();

            let data_buf = &mut buf[start_idx..];
            data_buf[..c.len()].copy_from_slice(c);
            out.extend_from_slice(encode_hex_buf(&mut buf, start_idx).unwrap());
        }

        let expect = (0..=255).map(|b| format!("{:02X?}", b)).collect::<String>();

        assert_eq!(out, expect.as_bytes())
    }
}
