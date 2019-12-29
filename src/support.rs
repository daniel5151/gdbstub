pub trait ToFromLEBytes: Sized + Copy {
    /// Create [Self] from an array of little-endian order bytes.
    /// Returns None if byte array is too short.
    /// The array can be longer than required (excess bytes are ignored).
    fn from_le_bytes(bytes: &[u8]) -> Option<Self>;

    /// Convert [Self] into an array of little-endian order bytes.
    /// Returns None if byte array is too short.
    /// The array can be longer than required.
    fn to_le_bytes(self, bytes: &mut [u8]) -> Option<usize>;
}

impl ToFromLEBytes for u8 {
    fn from_le_bytes(buf: &[u8]) -> Option<Self> {
        buf.get(0).copied()
    }

    fn to_le_bytes(self, buf: &mut [u8]) -> Option<usize> {
        buf.get_mut(0).map(|x| *x = self).map(|_| 1)
    }
}

macro_rules! impl_ToFromLEBytes {
    ($($type:ty),*) => {$(
        impl ToFromLEBytes for $type {
            fn from_le_bytes(buf: &[u8]) -> Option<Self> {
                if buf.len() < core::mem::size_of::<Self>() {
                    return None;
                }

                let mut b = [0; core::mem::size_of::<Self>()];
                b.copy_from_slice(&buf[..core::mem::size_of::<Self>()]);

                Some(Self::from_le_bytes(b))
            }

            fn to_le_bytes(self, buf: &mut [u8]) -> Option<usize> {
                if buf.len() < core::mem::size_of::<Self>() {
                    return None;
                }

                buf[..core::mem::size_of::<Self>()].copy_from_slice(&self.to_le_bytes());

                Some(core::mem::size_of::<Self>())
            }
        })*
    };
}

impl_ToFromLEBytes! { u16, u32, u64, u128 }
