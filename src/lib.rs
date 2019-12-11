//! ## Features
//!
//! - `std`:
//!   - Provides `impl Connection` for several common types (e.g: TcpStream)
//!   - Outputs protocol responses via `trace!`

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

mod connection_impls;
mod error;
mod protocol;
mod stub;

pub use error::Error;
pub use stub::GdbStub;

/// The set of operations that a GDB target needs to implement.
pub trait Target {
    /// The target architecture's pointer size
    type Usize: FromLEBytes;
    /// A target-specific unrecoverable error, which will be propagated
    /// through the GdbStub
    type Error;

    /// Return a target's features.xml file.
    ///
    /// See https://sourceware.org/gdb/current/onlinedocs/gdb/Target-Description-Format.html
    ///
    /// Protip: This can be used to have `gdb-multiarch` autodetect your target.
    /// Getting this up-and running might be as simple as returning:
    ///
    /// ```
    /// r#"
    /// <target version="1.0">
    ///     <architecture>your_arch_here</architecture>
    /// </target>"#
    /// ```
    fn target_description_xml() -> Option<&'static str> {
        None
    }

    /// Perform a single "step" of the target CPU.
    ///
    /// The provided `log_mem_access` function should be called each time a
    /// memory location is accessed.
    fn step(
        &mut self,
        log_mem_access: impl FnMut(Access<Self::Usize>),
    ) -> Result<TargetState, Self::Error>;

    /// Read the target's registers.
    ///
    /// The registers should be read in the order specified in the target's
    /// `target.xml`. The provided `push_reg` function should be called with the
    /// register's value.
    fn read_registers(&mut self, push_reg: impl FnMut(&[u8]));

    /// Read bytes from the specified address range
    fn read_addrs(&mut self, addr: core::ops::Range<Self::Usize>, val: impl FnMut(u8));
}

#[derive(Debug)]
pub enum AccessKind {
    Read,
    Write,
}

#[derive(Debug)]
pub struct Access<U> {
    pub kind: AccessKind,
    pub addr: U,
    pub val: u8,
}

// TODO: explore if TargetState is really necissary...
#[derive(Debug, PartialEq, Eq)]
pub enum TargetState {
    Running,
    Halted,
}

/// A trait similar to `impl Read + Write`, albeit without any dependencies on
/// std::io.
///
/// The default implementation of `read_nonblocking` falls-back to `read`. If
/// non-blocking reads are possible, you should provide your own implementation.
///
/// Enabling the `std` feature will automatically implement `Connection` on
/// several common types (such as TcpStream).
pub trait Connection {
    type Error;

    /// Read a single byte.
    fn read(&mut self) -> Result<u8, Self::Error>;

    /// Write a single byte.
    fn write(&mut self, byte: u8) -> Result<(), Self::Error>;

    /// Try to read a single byte, returning None if no data is available.
    fn read_nonblocking(&mut self) -> Result<Option<u8>, Self::Error> {
        self.read().map(Some)
    }

    /// Read the exact number of bytes required to fill buf,
    /// blocking if necessary.
    fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), Self::Error> {
        buf.iter_mut().try_for_each(|b| {
            *b = self.read()?;
            Ok(())
        })
    }
}

/// A simple trait that enables a type to be constructed from a slice of little
/// endian bytes. It is automatically implemented for u8 through u128.
pub trait FromLEBytes: Sized {
    /// Create [Self] from an array of little-endian order bytes.
    /// Returns None if byte array is too short.
    /// The array can be longer than required (truncating the result).
    fn from_le_bytes(bytes: &[u8]) -> Option<Self>;
}

impl FromLEBytes for u8 {
    fn from_le_bytes(buf: &[u8]) -> Option<Self> {
        buf.get(0).copied()
    }
}

macro_rules! impl_FromLEBytes {
    ($($type:ty),*) => {$(
        impl FromLEBytes for $type {
            fn from_le_bytes(buf: &[u8]) -> Option<Self> {
                if buf.len() < core::mem::size_of::<Self>() {
                    return None;
                }

                let mut b = [0; core::mem::size_of::<Self>()];
                b.copy_from_slice(&buf[..core::mem::size_of::<Self>()]);
                Some(Self::from_le_bytes(b))
            }
        })*
    };
}

impl_FromLEBytes! { u16, u32, u64, u128 }
