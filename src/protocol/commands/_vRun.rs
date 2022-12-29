use super::prelude::*;

use crate::protocol::common::lists::ArgListHex;

#[derive(Debug)]
pub struct vRun<'a> {
    pub filename: Option<&'a [u8]>,
    pub args: ArgListHex<'a>,
}

impl<'a> ParseCommand<'a> for vRun<'a> {
    #[inline(always)]
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        let body = buf.into_body();

        let mut body = body.splitn_mut(3, |b| *b == b';');

        let _first_semi = body.next()?;
        let filename = match decode_hex_buf(body.next()?).ok()? {
            [] => None,
            s => Some(s as &[u8]),
        };
        let args = body.next().unwrap_or(&mut []); // args are optional

        Some(vRun {
            filename,
            args: ArgListHex::from_packet(args)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! test_buf {
        ($bufname:ident, $body:literal) => {
            let mut test = $body.to_vec();
            let mut buf = PacketBuf::new_with_raw_body(&mut test).unwrap();
            if !buf.strip_prefix(b"vRun") {
                panic!("invalid test");
            }
            let $bufname = buf;
        };
    }

    #[test]
    fn valid_vRun_foobarbaz() {
        test_buf!(buf, b"vRun;;666f6f;626172;62617a");

        let pkt = vRun::from_packet(buf).unwrap();
        let args = pkt.args.into_iter().collect::<Vec<_>>();

        assert_eq!(pkt.filename, None);
        assert_eq!(args, &[b"foo", b"bar", b"baz"]);
    }

    #[test]
    fn valid_vRun_noname() {
        test_buf!(buf, b"vRun;");

        let pkt = vRun::from_packet(buf).unwrap();
        let args = pkt.args.into_iter().collect::<Vec<_>>();

        assert_eq!(pkt.filename, None);
        assert_eq!(args, &[] as &[&[u8]]);
    }

    #[test]
    fn valid_vRun_noargs() {
        test_buf!(buf, b"vRun;74657374");

        let pkt = vRun::from_packet(buf).unwrap();
        let args = pkt.args.into_iter().collect::<Vec<_>>();

        assert_eq!(pkt.filename, Some(&b"test"[..]));
        assert_eq!(args, &[] as &[&[u8]]);
    }

    #[test]
    fn valid_vRun_args() {
        test_buf!(buf, b"vRun;74657374;74657374");

        let pkt = vRun::from_packet(buf).unwrap();
        let args = pkt.args.into_iter().collect::<Vec<_>>();

        assert_eq!(pkt.filename, Some(&b"test"[..]));
        assert_eq!(args, &[b"test"]);
    }

    #[test]
    fn invalid_vRun_args() {
        test_buf!(buf, b"vRun;74657374;nothex");

        assert!(vRun::from_packet(buf).is_none());
    }

    #[test]
    fn invalid_vRun() {
        test_buf!(buf, b"vRun;nothex;nothex");

        assert!(vRun::from_packet(buf).is_none());
    }
}
