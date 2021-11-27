//! Common types and definitions used across `gdbstub`.

mod signal;

pub use self::signal::Signal;

/// Thread ID
pub type Tid = core::num::NonZeroUsize;

/// Process ID
pub type Pid = core::num::NonZeroUsize;
