mod hex;
mod thread_id;

pub use hex::*;
pub use thread_id::*;

/// Lightweight wrapper around `&[u8]` which denotes that the contained data is
/// a ASCII string.
#[derive(Debug)]
#[repr(transparent)]
pub struct Bstr<'a>(&'a [u8]);

impl<'a> From<&'a [u8]> for Bstr<'a> {
    fn from(s: &'a [u8]) -> Bstr<'a> {
        Bstr(s)
    }
}

impl<'a> From<Bstr<'a>> for &'a [u8] {
    fn from(s: Bstr<'a>) -> &'a [u8] {
        s.0
    }
}

impl AsRef<[u8]> for Bstr<'_> {
    fn as_ref(&self) -> &[u8] {
        self.0
    }
}
