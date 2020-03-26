//! An implementation of the
//! [GDB Remote Serial Protocol](https://sourceware.org/gdb/onlinedocs/gdb/Remote-Protocol.html#Remote-Protocol)
//! in Rust, primarily for use in emulators.
//!
//! `gdbstub` tries to make as few assumptions as possible about your target's
//! architecture, and aims to provide a "drop-in" way to add GDB support into a
//! project, _without_ requiring any large refactoring / ownership juggling.
//!
//! `gdbstub` is `no_std` by default, though it does have a dependency on
//! `alloc`.
//!
//! ## Features
//!
//! - `std` - (disabled by default)
//!   - Implements [`Connection`](trait.Connection.html) for [`std::net::TcpStream`](https://doc.rust-lang.org/std/net/struct.TcpStream.html)
//!   - Implements [`std::error::Error`](https://doc.rust-lang.org/std/error/trait.Error.html)
//!     for `gdbstub::Error`
//!   - Outputs protocol responses via `log::trace!`

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

/// Describes a target system which can be debugged using
/// [`GdbStub`](struct.GdbStub.html).
///
/// This trait describes the architecture and capabilities of a target
/// system, and provides an interface for `GdbStub` to modify and control the
/// system's state.
///
/// Several of the trait's "Provided methods" can be overwritten to
/// enable certain advanced GDB debugging features. For example, the
/// [`target_description_xml`](#method.target_description_xml) method can be
/// overwritten to enable automatic architecture detection.
///
/// ### What's `<target>.xml`?
///
/// Some required methods rely on target-specific information which can only be
/// found in GDB's internal `<target>.xml` files. For example, a basic 32-bit
/// ARM target uses the register layout described in the
///  [`arm-core.xml`](https://github.com/bminor/binutils-gdb/blob/master/gdb/features/arm/arm-core.xml)
/// file.
pub trait Target {
    /// The target architecture's pointer size (e.g: `u32` on a 32-bit system).
    type Usize: PrimInt + Unsigned + Debug;

    /// A target-specific fatal error.
    type Error;

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
    /// The registers should be read in the order specified in the
    /// [`<target>.xml`](#whats-targetxml). The provided `push_reg` function
    /// should be called with the register's value.
    // e.g: for ARM: binutils-gdb/blob/master/gdb/features/arm/arm-core.xml
    // TODO: modify signature to return Result<(), Self::Error>
    fn read_registers(&mut self, push_reg: impl FnMut(&[u8]));

    /// Write the target's registers.
    ///
    /// The bytes are provided in the order specified in the target's registers
    /// are provided in the order specified in the
    /// [`<target>.xml`](#whats-targetxml).
    ///
    /// e.g: for ARM: binutils-gdb/blob/master/gdb/features/arm/arm-core.xml
    // TODO: modify signature to return Result<(), Self::Error>
    fn write_registers(&mut self, regs: &[u8]);

    /// Read the target's current PC.
    fn read_pc(&mut self) -> Self::Usize;

    /// Read bytes from the specified address range.
    // TODO: modify signature to return Result<(), Self::Error>
    fn read_addrs(&mut self, addr: core::ops::Range<Self::Usize>, val: impl FnMut(u8));

    /// Write bytes to the specified address range.
    // TODO: modify signature to return Result<(), Self::Error>
    fn write_addrs(&mut self, get_addr_val: impl FnMut() -> Option<(Self::Usize, u8)>);

    /// Return the platform's `features.xml` file.
    ///
    /// Implementing this method enables `gdb` to automatically detect the
    /// target's architecture, saving the hassle of having to run `set
    /// architecture <arch>` when starting a debugging session.
    ///
    /// These descriptions can be quite succinct. For example, the target
    /// description for an `armv4t` platform can be as simple as:
    ///
    /// ```
    /// Some(r#"
    /// <target version="1.0">
    ///     <architecture>armv4t</architecture>
    /// </target>"#)
    /// ```
    ///
    /// See the [GDB docs](https://sourceware.org/gdb/current/onlinedocs/gdb/Target-Description-Format.html)
    /// for details on the target description XML format.
    fn target_description_xml() -> Option<&'static str> {
        None
    }
}

/// The kind of memory access being performed
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccessKind {
    /// Read
    Read,
    /// Write
    Write,
}

/// Describes a memory access.
#[derive(Clone, Copy, Debug)]
pub struct Access<U> {
    /// The kind of memory access (Read or Write).
    pub kind: AccessKind,
    /// The associated address.
    pub addr: U,
    /// The byte that was read / written.
    pub val: u8,
}

/// The underlying system's execution state.
// TODO: explore if TargetState is really necessary...
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TargetState {
    /// Running
    Running,
    /// Halted
    Halted,
}

/// A trait for reading / writing bytes across some transport layer.
///
/// Enabling the optional `std` feature provides implementations of `Connection`
/// on several common std types (such as `std::net::TcpStream`).
///
/// _Note_: the default implementation of `read_nonblocking` will fall-back to
/// the blocking `read` implementation. If non-blocking reads are possible, you
/// should provide your own implementation.
// TODO: remove this silly default read_nonblocking implementation!
pub trait Connection {
    /// Transport-specific error type.
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
