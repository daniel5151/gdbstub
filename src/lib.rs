mod commands;
mod connection_impls;
mod error;
mod stub;

pub use commands::{Command, Error as CommandParseError};
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

    /// Perform a single "step" of the target CPU, recording any memory accesses
    /// in the `mem_accesses` vector. The return value indicates
    fn step(
        &mut self,
        mem_accesses: &mut Vec<(AccessKind, Self::Usize, u8)>,
    ) -> Result<TargetState, Self::Error>;
}

#[derive(Debug)]
pub enum AccessKind {
    Read,
    Write,
}

#[derive(PartialEq, Eq)]
pub enum TargetState {
    Running,
    Halted,
}

/// A trait similar to `impl Read + Write`, albeit without any dependency on
/// std::io, and with some additional helper methods.
///
/// The default implementation of `read_nonblocking` falls-back to `read`. If
/// non-blocking reads are possible, you should provide your own implementation.
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
