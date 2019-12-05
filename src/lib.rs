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
    type Usize;
    /// A target-specific unrecoverable error, which should be propagated
    /// through the GdbStub
    type Error;

    // /// Read a byte from a memory address
    // fn read(&mut self, addr: Self::Usize) -> u8;
    // /// Write a byte to a memory address
    // fn write(&mut self, addr: Self::Usize, val: u8);

    /// Perform a single "step" of the target CPU.
    /// The provided `log_mem_access` function should be called each time a
    /// memory location is accessed.
    fn step(
        &mut self,
        log_mem_access: impl FnMut(Access<Self::Usize>),
    ) -> Result<TargetState, Self::Error>;
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

#[derive(PartialEq, Eq)]
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
