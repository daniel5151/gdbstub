use btoi::{btou_radix, ParseIntegerError};
use num_traits::{CheckedAdd, CheckedMul, FromPrimitive, Zero};

#[inline]
pub fn decode_hex<I>(buf: &[u8]) -> Result<I, ParseIntegerError>
where
    I: FromPrimitive + Zero + CheckedAdd + CheckedMul,
{
    btou_radix(buf, 16)
}

fn ascii2byte(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

/// Decode a hex string into a mutable bytes slice _in place_.
pub fn decode_hex_buf<'a>(buf: &'a mut [u8]) -> Result<&'a mut [u8], &'static str> {
    const MUST_BE_ASCII: &str = "buf must only contain ASCII hexdigits";
    const EVEN_LEN: &str = "buf must have even number of bytes";

    if buf.len() % 2 != 0 {
        return Err(EVEN_LEN);
    }

    let decoded_len = buf.len() / 2;
    for i in 0..decoded_len {
        let b = ascii2byte(buf[i * 2]).ok_or(MUST_BE_ASCII)? << 4
            | ascii2byte(buf[i * 2 + 1]).ok_or(MUST_BE_ASCII)?;
        buf[i] = b as u8;
    }

    Ok(&mut buf[..decoded_len])
}
