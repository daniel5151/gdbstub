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
