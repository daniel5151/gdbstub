//! Common types and definitions used across `gdbstub`.

mod signal;

pub use self::signal::Signal;
use core::num::NonZeroUsize;

/// Thread ID (as viewed by GDB)
///
/// The choice to use a [`NonZeroUsize`] stems from the [GDB RSP Packet
/// documentation], which states that thread IDs are "positive numbers with a
/// target-specific interpretation".
///
/// Target implementations may wish to map `Tid`s to/from their own
/// target-specific thread ID type. (e.g: an emulator might treat `Tid` as a CPU
/// index).
///
/// [GDB RSP Packet documentation]:
///     https://sourceware.org/gdb/current/onlinedocs/gdb.html/Packets.html#Packets
pub type Tid = NonZeroUsize;

/// Process ID (as viewed by GDB)
///
/// The choice to use a [`NonZeroUsize`] stems from the [GDB RSP Packet
/// documentation], which states that process IDs are "positive numbers with a
/// target-specific interpretation".
///
/// Target implementations may wish to map `Pid`s to/from their own
/// target-specific process ID type.
///
/// [GDB RSP Packet documentation]:
///     https://sourceware.org/gdb/current/onlinedocs/gdb.html/Packets.html#Packets
pub type Pid = NonZeroUsize;

/// Endianness.
///
/// This is used to report target endianness to the debugger as a
/// response to certain commands.
#[derive(Clone, Copy, Debug)]
pub enum Endianness {
    /// Big-endian.
    Big,
    /// Little-endian.
    Little,
}
