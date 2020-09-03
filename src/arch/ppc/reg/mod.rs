//! Registers for PowerPC architectures

use core::convert::TryInto;

mod core32;

pub use core32::PowerPcCoreRegs;

/// A value stored in a PowerPC vector register
#[repr(transparent)]
#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub struct PpcVector(u128);

macro_rules! concat_arr4 {
    ($($x:expr),*) => {
        [
            $(
                $x[0], $x[1], $x[2], $x[3],
            )*
        ]
    };
}

fn concat4x4(a: [u8; 4], b: [u8; 4], c: [u8; 4], d: [u8; 4]) -> [u8; 16] {
    concat_arr4!(a, b, c, d)
}

macro_rules! concat_arr2 {
    ($($x:expr),*) => {
        [
            $(
                $x[0], $x[1],
            )*
        ]
    };
}

fn concat2x8(x: [[u8; 2]; 8]) -> [u8; 16] {
    concat_arr2!(x[0], x[1], x[2], x[3], x[4], x[5], x[6], x[7])
}

impl From<u128> for PpcVector {
    fn from(x: u128) -> Self {
        PpcVector(x)
    }
}

impl From<[f32; 4]> for PpcVector {
    fn from(x: [f32; 4]) -> Self {
        PpcVector(u128::from_be_bytes(concat4x4(
            x[3].to_be_bytes(),
            x[2].to_be_bytes(),
            x[1].to_be_bytes(),
            x[0].to_be_bytes(),
        )))
    }
}

impl From<[u32; 4]> for PpcVector {
    fn from(x: [u32; 4]) -> Self {
        PpcVector(u128::from_be_bytes(concat4x4(
            x[3].to_be_bytes(),
            x[2].to_be_bytes(),
            x[1].to_be_bytes(),
            x[0].to_be_bytes(),
        )))
    }
}

impl From<[u16; 8]> for PpcVector {
    fn from(x: [u16; 8]) -> Self {
        PpcVector(u128::from_be_bytes(concat2x8([
            x[7].to_be_bytes(),
            x[6].to_be_bytes(),
            x[5].to_be_bytes(),
            x[4].to_be_bytes(),
            x[3].to_be_bytes(),
            x[2].to_be_bytes(),
            x[1].to_be_bytes(),
            x[0].to_be_bytes(),
        ])))
    }
}

impl From<[u8; 0x10]> for PpcVector {
    fn from(x: [u8; 0x10]) -> Self {
        PpcVector(u128::from_le_bytes(x))
    }
}

impl Into<u128> for PpcVector {
    fn into(self) -> u128 {
        self.0
    }
}

impl Into<[f32; 4]> for PpcVector {
    fn into(self) -> [f32; 4] {
        let bytes = self.0.to_be_bytes();
        let mut floats = bytes
            .chunks_exact(4)
            .map(|x| f32::from_be_bytes(x.try_into().unwrap()));

        let x0 = floats.next().unwrap();
        let x1 = floats.next().unwrap();
        let x2 = floats.next().unwrap();
        let x3 = floats.next().unwrap();

        [x3, x2, x1, x0]
    }
}

impl Into<[u32; 4]> for PpcVector {
    fn into(self) -> [u32; 4] {
        let bytes = self.0.to_be_bytes();
        let mut ints = bytes
            .chunks_exact(4)
            .map(|x| u32::from_be_bytes(x.try_into().unwrap()));

        let x0 = ints.next().unwrap();
        let x1 = ints.next().unwrap();
        let x2 = ints.next().unwrap();
        let x3 = ints.next().unwrap();

        [x3, x2, x1, x0]
    }
}

impl Into<[u16; 8]> for PpcVector {
    fn into(self) -> [u16; 8] {
        let bytes = self.0.to_be_bytes();
        let mut ints = bytes
            .chunks_exact(2)
            .map(|x| u16::from_be_bytes(x.try_into().unwrap()));

        let x0 = ints.next().unwrap();
        let x1 = ints.next().unwrap();
        let x2 = ints.next().unwrap();
        let x3 = ints.next().unwrap();
        let x4 = ints.next().unwrap();
        let x5 = ints.next().unwrap();
        let x6 = ints.next().unwrap();
        let x7 = ints.next().unwrap();

        [x7, x6, x5, x4, x3, x2, x1, x0]
    }
}

impl Into<[u8; 0x10]> for PpcVector {
    fn into(self) -> [u8; 0x10] {
        self.0.to_le_bytes()
    }
}

#[cfg(test)]
mod ppc_tests {
    use super::PpcVector;

    #[test]
    fn ppc_vector_f32_round_trip() {
        let x = [1.0, 2.0, 3.0, 4.0];
        let y: PpcVector = x.into();
        let z: [f32; 4] = y.into();

        assert_eq!(x, z);
    }

    #[test]
    fn ppc_vector_u32_round_trip() {
        let x = [1, 2, 3, 4];
        let y: PpcVector = x.into();
        let z: [u32; 4] = y.into();

        assert_eq!(x, z);
    }

    #[test]
    fn ppc_vector_u16_round_trip() {
        let x = [1, 2, 3, 4, 5, 6, 7, 8];
        let y: PpcVector = x.into();
        let z: [u16; 8] = y.into();

        assert_eq!(x, z);
    }

    #[test]
    fn ppc_vector_bytes_round_trip() {
        let x = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0xa, 0xb, 0xc, 0xd, 0xe, 0xf];
        let y: PpcVector = x.into();
        let z: [u8; 0x10] = y.into();

        assert_eq!(x, z);
    }

    #[test]
    fn ppc_vector_u128_round_trip() {
        let x = 12345678_u128;
        let y: PpcVector = x.into();
        let z: u128 = y.into();

        assert_eq!(x, z);
    }
}
