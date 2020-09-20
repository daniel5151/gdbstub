use super::prelude::*;

#[derive(Debug)]
pub struct vRun<'a> {
    pub filename: Option<&'a [u8]>,
    pub args: Args<'a>,
}

#[derive(Debug)]
pub struct Args<'a>(&'a mut [u8]);

impl<'a> Args<'a> {
    pub fn into_iter(self) -> impl Iterator<Item = &'a [u8]> + 'a {
        self.0
            .split_mut(|b| *b == b';')
            // the `from_packet` method guarantees that the args are valid hex ascii, so this should
            // method should never fail.
            .map(|raw| decode_hex_buf(raw).unwrap_or(&mut []))
            .map(|s| s as &[u8])
            .filter(|s| !s.is_empty())
    }
}

impl<'a> ParseCommand<'a> for vRun<'a> {
    fn __protocol_hint(target: &mut impl Target) -> bool {
        target.extended_mode().is_some()
    }

    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        let body = buf.into_body();

        let mut body = body.splitn_mut(3, |b| *b == b';');

        let _first_semi = body.next()?;
        let filename = match decode_hex_buf(body.next()?).ok()? {
            [] => None,
            s => Some(s as &[u8]),
        };
        let args = body.next().unwrap_or(&mut []); // args are optional

        // validate that args have valid hex encoding (with ';' delimiters).
        // this removes all the error handling from the lazy `Args` iterator.
        if args.iter().any(|b| !(is_hex(*b) || *b == b';')) {
            return None;
        }

        Some(vRun {
            filename,
            args: Args(args),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! test_buf {
        ($bufname:ident, $body:literal) => {
            let mut test = $body.to_vec();
            let buf = PacketBuf::new_with_raw_body(&mut test).unwrap();
            let $bufname = buf.trim_start_body_bytes(b"vRun".len());
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
