use super::prelude::*;
use crate::protocol::common::hex::decode_bin_buf;

#[derive(Debug)]
pub struct vFlashWrite<'a> {
    pub addr: &'a [u8],
    pub val: &'a [u8],
}

impl<'a> ParseCommand<'a> for vFlashWrite<'a> {
    #[inline(always)]
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        let body = buf.into_body();

        let mut body = body.splitn_mut(3, |&b| b == b':');
        let _first_colon = body.next()?;
        let addr = decode_hex_buf(body.next()?)
            .ok()
            .filter(|a| !a.is_empty())?;
        let val = decode_bin_buf(body.next()?)?;

        Some(vFlashWrite { addr, val })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! test_buf {
        ($bufname:ident, $body:literal) => {
            let mut test = $body.to_vec();
            let mut buf = PacketBuf::new_with_raw_body(&mut test).unwrap();
            if !buf.strip_prefix(b"vFlashWrite") {
                panic!("invalid test");
            }
            let $bufname = buf;
        };
    }

    #[test]
    fn valid_vFlashWrite() {
        test_buf!(
            buf,
            b"vFlashWrite:08000000:\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0A"
        );

        let pkt = vFlashWrite::from_packet(buf).unwrap();

        assert_eq!(pkt.addr, [0x08, 0, 0, 0]);
        assert_eq!(pkt.val, [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    }

    #[test]
    fn invalid_vFlashWrite_wrong_address() {
        test_buf!(
            buf,
            b"vFlashWrite:abcdefg:\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0A"
        );

        assert!(vFlashWrite::from_packet(buf).is_none())
    }

    #[test]
    fn invalid_vFlashWrite_missing_data() {
        test_buf!(buf, b"vFlashWrite:abcdefg:");

        assert!(vFlashWrite::from_packet(buf).is_none())
    }

    #[test]
    fn invalid_vFlashWrite_missing_address() {
        test_buf!(buf, b"vFlashWrite:");

        assert!(vFlashWrite::from_packet(buf).is_none())
    }
}
