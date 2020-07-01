use core::iter::FusedIterator;

/// Iterator over ASCII buffers which converts pairs of hex-chars into u8.
///
/// ```ignore
/// assert_eq!(HexDecoder::new("deadbeef").collect::<Vec<_>>(), &[0xde, 0xad, 0xbe, 0xef])
/// ```
#[derive(PartialEq, Eq, Debug)]
pub struct HexDecoder<'a> {
    i: usize,
    buf: &'a [u8],
}

impl<'a> HexDecoder<'a> {
    pub fn new(buf: &'a str) -> Result<HexDecoder<'a>, &'static str> {
        if buf.as_bytes().len() % 2 != 0 {
            return Err("buf must have even number of chars");
        }

        if !buf.as_bytes().iter().all(|b| b.is_ascii_hexdigit()) {
            return Err("buf must only contain ASCII hexdigits");
        }

        Ok(HexDecoder {
            i: 0,
            buf: buf.as_bytes(),
        })
    }
}

impl<'a> FusedIterator for HexDecoder<'a> {}
impl<'a> Iterator for HexDecoder<'a> {
    type Item = u8;

    fn next(&mut self) -> Option<u8> {
        if self.i >= self.buf.len() {
            return None;
        }

        // unwrap can't panic, since the constructor checks that all bytes are ascii
        // hexdigits
        let ret = (self.buf[self.i] as char).to_digit(16).unwrap() << 4
            | (self.buf[self.i + 1] as char).to_digit(16).unwrap();
        self.i += 2;
        Some(ret as u8)
    }
}

/// Decode a hex string into a mutable bytes slice _in place_.
pub fn decode_hex<'a>(buf: &'a mut [u8]) -> Result<&'a mut [u8], &'static str> {
    const MUST_BE_ASCII: &str = "buf must only contain ASCII hexdigits";
    const EVEN_LEN: &str = "buf must have even number of bytes";

    if buf.len() % 2 != 0 {
        return Err(EVEN_LEN);
    }

    let decoded_len = buf.len() / 2;
    for i in 0..decoded_len {
        let b = (buf[i * 2] as char).to_digit(16).ok_or(MUST_BE_ASCII)? << 4
            | (buf[i * 2 + 1] as char).to_digit(16).ok_or(MUST_BE_ASCII)?;
        buf[i] = b as u8;
    }

    Ok(&mut buf[..decoded_len])
}
