/// A trait for working with structs as big-endian byte arrays. Automatically
/// implemented for all built-in signed/unsigned integers.
pub trait BeBytes: Sized {
    /// Write the memory representation of `self` as a byte array in
    /// big-endian (network) byte order into the provided buffer.
    fn to_be_bytes(self, buf: &mut [u8]) -> Option<usize>;

    /// Parse `self` from a byte array in big-endian (network) byte order.
    /// Returns None upon overflow.
    fn from_be_bytes(buf: &[u8]) -> Option<Self>;
}

macro_rules! impl_to_be_bytes {
    ($($num:ty)*) => {
        $(
            impl BeBytes for $num {
                fn to_be_bytes(self, buf: &mut [u8]) -> Option<usize> {
                    let len = core::mem::size_of::<$num>();
                    if buf.len() < len {
                        return None
                    }
                    buf[..len].copy_from_slice(&<$num>::to_be_bytes(self));
                    Some(len)
                }

                fn from_be_bytes(buf: &[u8]) -> Option<Self> {
                    use core::convert::TryInto;
                    let len = core::mem::size_of::<$num>();

                    let buf = if buf.len() > len {
                        let (extra, buf) = buf.split_at(buf.len() - len);
                        if extra.iter().any(|&b| b != 0) {
                            return None
                        }
                        buf
                    } else {
                        buf
                    };
                    Some(<$num>::from_be_bytes(buf.try_into().ok()?))
                }
            }
        )*
    };
}

impl_to_be_bytes!(u8 u16 u32 u64 u128 usize i8 i16 i32 i64 i128 isize);
