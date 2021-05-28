use crate::protocol::common::hex::{decode_hex_buf, is_hex};

/// A wrapper type around a list of hex encoded arguments separated by `;`.
#[derive(Debug)]
pub struct ArgListHex<'a>(&'a mut [u8]);

impl<'a> ArgListHex<'a> {
    pub fn from_packet(args: &'a mut [u8]) -> Option<Self> {
        // validate that args have valid hex encoding (with ';' delimiters).
        // this removes all the error handling from the lazy `Args` iterator.
        if args.iter().any(|b| !(is_hex(*b) || *b == b';')) {
            return None;
        }
        Some(Self(args))
    }

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
