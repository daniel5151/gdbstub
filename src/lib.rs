//! ## Features
//!
//! - `std`:
//!   - Provides `impl Connection` for several common types (e.g: TcpStream)
//!   - Outputs protocol responses via `trace!`

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use core::fmt::Debug;

use num_traits::{PrimInt, Unsigned};

mod connection_impls;
mod error;
mod protocol;
mod stub;

pub use error::Error;
pub use stub::GdbStub;

/// The set of operations that a GDB target needs to implement.
pub trait Target {
    /// The target architecture's pointer size.
    type Usize: PrimInt + Unsigned + Debug;

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

    /// Write the target's registers.
    ///
    /// The bytes are provided in the order specified in the target's registers
    /// are provided in the order specified in the target's `target.xml`.
    fn write_registers(&mut self, regs: &[u8]);

    /// Read the target's current PC
    fn read_pc(&mut self) -> Self::Usize;

    /// Read bytes from the specified address range
    fn read_addrs(&mut self, addr: core::ops::Range<Self::Usize>, val: impl FnMut(u8));

    /// Write bytes to the specified address range
    fn write_addrs(&mut self, get_addr_val: impl FnMut() -> Option<(Self::Usize, u8)>);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccessKind {
    Read,
    Write,
}

#[derive(Clone, Debug)]
pub struct Access<U> {
    pub kind: AccessKind,
    pub addr: U,
    pub val: u8,
}

// TODO: explore if TargetState is really necessary...
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

    /// Read the exact number of bytes required to fill buf, blocking if
    /// necessary.
    fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), Self::Error> {
        buf.iter_mut().try_for_each(|b| {
            *b = self.read()?;
            Ok(())
        })
    }
}
