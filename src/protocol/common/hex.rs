use num_traits::{CheckedAdd, CheckedMul, FromPrimitive, Zero};

#[derive(Debug)]
pub enum DecodeHexError {
    NotAscii,
    Empty,
    Overflow,
    InvalidOutput,
}

/// Decode a GDB dex string into the specified integer.
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

#[derive(Debug)]
pub enum DecodeHexBufError {
    NotAscii,
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

/// Check if the byte `c` is a valid GDB hex digit `[0-9][a-f][A-F][xX]`
pub fn is_hex(c: u8) -> bool {
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
// TODO: rewrite this method to elide bound checks
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
        buf[i] = b as u8;
    }

    Ok(&mut base_buf[..decoded_len + odd_adust])
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

    #[test]
    fn decode_hex_buf_odd() {
        let mut payload = b"ffffff4".to_vec();
        let res = decode_hex_buf(&mut payload).unwrap();
        assert_eq!(res, [0xf, 0xff, 0xff, 0xf4]);
    }

    #[test]
    fn decode_hex_buf_2() {
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
}
