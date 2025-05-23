use super::prelude::*;

#[derive(Debug)]
pub struct vFlashErase<'a> {
    pub addr: &'a [u8],
    pub length: &'a [u8],
}

impl<'a> ParseCommand<'a> for vFlashErase<'a> {
    #[inline(always)]
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        let body = buf.into_body();

        let mut body = body.splitn_mut(3, |&b| b == b',' || b == b':');
        let _first_colon = body.next()?;
        let addr = decode_hex_buf(body.next()?).ok()?;
        let length = decode_hex_buf(body.next()?)
            .ok()
            .filter(|l| !l.is_empty())?;
        Some(Self { addr, length })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! test_buf {
        ($bufname:ident, $body:literal) => {
            let mut test = $body.to_vec();
            let mut buf = PacketBuf::new_with_raw_body(&mut test).unwrap();
            if !buf.strip_prefix(b"vFlashErase") {
                panic!("invalid test");
            }
            let $bufname = buf;
        };
    }

    #[test]
    fn valid_vFlashErase() {
        test_buf!(buf, b"vFlashErase:08000000,00004000");

        let pkt = vFlashErase::from_packet(buf).unwrap();

        assert_eq!(pkt.addr, [0x08, 0, 0, 0]);
        assert_eq!(pkt.length, [0, 0, 0x40, 0]);
    }

    #[test]
    fn invalid_vFlashErase_wrong_address() {
        test_buf!(buf, b"vFlashErase:abcdefg:00004000");

        assert!(vFlashErase::from_packet(buf).is_none());
    }

    #[test]
    fn invalid_vFlashErase_wrong_length() {
        test_buf!(buf, b"vFlashErase:08000000:abcdefg");

        assert!(vFlashErase::from_packet(buf).is_none());
    }

    #[test]
    fn invalid_vFlashErase_missing_address() {
        test_buf!(buf, b"vFlashErase:");

        assert!(vFlashErase::from_packet(buf).is_none());
    }

    #[test]
    fn invalid_vFlashErase_missing_length() {
        test_buf!(buf, b"vFlashErase:08000000:");

        assert!(vFlashErase::from_packet(buf).is_none());
    }
}
