use num_traits::{CheckedAdd, CheckedMul, FromPrimitive, Zero};

#[derive(Debug)]
pub enum DecodeHexError {
    NotAscii,
    Empty,
    Overflow,
    InvalidOutput,
}

/// Decode a GDB hex string into the specified integer.
///
/// GDB hex strings may include "xx", which represent "missing" data. This
/// method simply treats "xx" as 0x00.
pub fn decode_hex<I>(buf: &[u8]) -> Result<I, DecodeHexError>
where
    I: FromPrimitive + Zero + CheckedAdd + CheckedMul,
{
    use DecodeHexError::*;

    let radix = I::from_u8(16).ok_or(InvalidOutput)?;

    if buf.is_empty() {
        return Err(Empty);
    }

    let mut result = I::zero();

    for &digit in buf {
        let x = I::from_u8(ascii2byte(digit).ok_or(NotAscii)?).ok_or(InvalidOutput)?;
        result = result.checked_mul(&radix).ok_or(Overflow)?;
        result = result.checked_add(&x).ok_or(Overflow)?
    }

    Ok(result)
}

/// Wrapper around a raw hex string. Enables "late" calls to `decode` from
/// outside the `crate::protocol` module.
#[derive(Debug, Clone, Copy)]
pub struct HexString<'a>(pub &'a [u8]);

impl HexString<'_> {
    pub fn decode<I>(&self) -> Result<I, DecodeHexError>
    where
        I: FromPrimitive + Zero + CheckedAdd + CheckedMul,
    {
        decode_hex(self.0)
    }
}

#[derive(Debug)]
pub enum DecodeHexBufError {
    NotAscii,
}

#[inline]
fn ascii2byte(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        b'x' | b'X' => Some(0),
        _ => None,
    }
}

/// Check if the byte `c` is a valid GDB hex digit `[0-9a-fA-FxX]`
#[inline]
pub fn is_hex(c: u8) -> bool {
    #[allow(clippy::match_like_matches_macro)] // mirror ascii2byte
    match c {
        b'0'..=b'9' => true,
        b'a'..=b'f' => true,
        b'A'..=b'F' => true,
        b'x' | b'X' => true,
        _ => false,
    }
}

/// Decode a GDB hex string into a byte slice _in place_.
///
/// GDB hex strings may include "xx", which represent "missing" data. This
/// method simply treats "xx" as 0x00.
// TODO: maybe don't blindly translate "xx" as 0x00?
#[cfg(not(feature = "paranoid_unsafe"))]
pub fn decode_hex_buf(base_buf: &mut [u8]) -> Result<&mut [u8], DecodeHexBufError> {
    use DecodeHexBufError::*;

    if base_buf.is_empty() {
        return Ok(&mut []);
    }

    let odd_adust = base_buf.len() % 2;
    if odd_adust != 0 {
        base_buf[0] = ascii2byte(base_buf[0]).ok_or(NotAscii)?;
    }

    let buf = &mut base_buf[odd_adust..];

    let decoded_len = buf.len() / 2;
    for i in 0..decoded_len {
        // SAFETY: rustc isn't smart enough to automatically elide these bound checks.
        //
        // If buf.len() == 0 or 1: trivially safe, since the for block is never taken
        // If buf.len() >= 2: the range of values for `i` is 0..(buf.len() / 2 - 1)
        let (hi, lo, b) = unsafe {
            (
                //    (buf.len() / 2 - 1) * 2
                // == (buf.len() - 2)
                // since buf.len() is >2, this is in-bounds
                *buf.get_unchecked(i * 2),
                //    (buf.len() / 2 - 1) * 2 + 1
                // == (buf.len() - 1)
                // since buf.len() is >2, this is in-bounds
                *buf.get_unchecked(i * 2 + 1),
                // since buf.len() is >2, (buf.len() / 2 - 1) is always in-bounds
                buf.get_unchecked_mut(i),
            )
        };

        let hi = ascii2byte(hi).ok_or(NotAscii)?;
        let lo = ascii2byte(lo).ok_or(NotAscii)?;
        *b = hi << 4 | lo;
    }

    // SAFETY: rustc isn't smart enough to automatically elide this bound check.
    //
    // Consider the different values (decoded_len + odd_adust) can take:
    //
    //  buf.len() | (decoded_len + odd_adust)
    // -----------|---------------------------
    //      0     | (0 + 0) == 0
    //      1     | (0 + 1) == 1
    //      2     | (1 + 0) == 1
    //      3     | (1 + 1) == 2
    //      4     | (2 + 0) == 2
    //      5     | (2 + 1) == 3
    //
    // Note that the computed index is always in-bounds.
    //
    // If I were still in undergrad, I could probably have whipped up a proper
    // mathematical proof by induction or whatnot, but hopefully this "proof by
    // example" ought to suffice.
    unsafe { Ok(base_buf.get_unchecked_mut(..decoded_len + odd_adust)) }
}

/// Decode a GDB hex string into a byte slice _in place_.
///
/// GDB hex strings may include "xx", which represent "missing" data. This
/// method simply treats "xx" as 0x00.
// TODO: maybe don't blindly translate "xx" as 0x00?
#[cfg(feature = "paranoid_unsafe")]
pub fn decode_hex_buf(base_buf: &mut [u8]) -> Result<&mut [u8], DecodeHexBufError> {
    use DecodeHexBufError::*;

    let odd_adust = base_buf.len() % 2;
    if odd_adust != 0 {
        base_buf[0] = ascii2byte(base_buf[0]).ok_or(NotAscii)?;
    }
    let buf = &mut base_buf[odd_adust..];

    let decoded_len = buf.len() / 2;
    for i in 0..decoded_len {
        let b = ascii2byte(buf[i * 2]).ok_or(NotAscii)? << 4
            | ascii2byte(buf[i * 2 + 1]).ok_or(NotAscii)?;
        buf[i] = b;
    }

    Ok(&mut base_buf[..decoded_len + odd_adust])
}

/// Decode GDB escaped binary bytes into origin bytes _in place_.
//
// Thanks reddit!
// https://www.reddit.com/r/rust/comments/110qzq9/any_idea_why_rust_isnt_able_to_elide_this_bounds/
pub fn decode_bin_buf(buf: &mut [u8]) -> Option<&mut [u8]> {
    let mut i = 0;
    let len = buf.len();
    for j in 0..len {
        if i >= len {
            return Some(&mut buf[..j]);
        }

        if buf[i] == b'}' {
            buf[j] = buf.get(i + 1)? ^ 0x20;
            i += 1;
        } else {
            buf[j] = buf[i];
        }
        i += 1;
    }

    Some(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_hex_buf_odd() {
        let mut payload = b"ffffff4".to_vec();
        let res = decode_hex_buf(&mut payload).unwrap();
        assert_eq!(res, [0xf, 0xff, 0xff, 0xf4]);
    }

    #[test]
    fn decode_hex_buf_even() {
        let mut payload = b"0123456789abcdef".to_vec();
        let res = decode_hex_buf(&mut payload).unwrap();
        assert_eq!(res, [0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef]);
    }

    #[test]
    fn decode_hex_buf_odd_alt() {
        let mut payload = b"12345".to_vec();
        let res = decode_hex_buf(&mut payload).unwrap();
        assert_eq!(res, [0x1, 0x23, 0x45]);
    }

    #[test]
    fn decode_hex_buf_short() {
        let mut payload = b"1".to_vec();
        let res = decode_hex_buf(&mut payload).unwrap();
        assert_eq!(res, [0x1]);
    }

    #[test]
    fn decode_hex_buf_empty() {
        let mut payload = b"".to_vec();
        let res = decode_hex_buf(&mut payload).unwrap();
        assert_eq!(res, []);
    }

    #[test]
    fn decode_bin_buf_escaped() {
        let mut payload = b"}\x03}\x04}]}\n".to_vec();
        let res = decode_bin_buf(&mut payload).unwrap();
        assert_eq!(res, [0x23, 0x24, 0x7d, 0x2a]);
    }
}
