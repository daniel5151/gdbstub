//! [`Target`](trait.Target.html) and it's various optional extension traits.
//!
//! ### What's with all the `<Self::Arch as Arch>::` syntax?
//!
//! Yeah, sorry about that!
//!
//! If [rust-lang/rust#38078](https://github.com/rust-lang/rust/issues/38078)
//! every gets fixed, `<Self::Arch as Arch>::Foo` will be simplified to just
//! `Self::Arch::Foo`.
//!
//! Until then, when implementing `Target`, it's recommended to use the concrete
//! type directly. e.g: on a 32-bit platform, instead of writing `<Self::Arch
//! as Arch>::Usize`, use `u32` directly.

use crate::arch::Arch;

pub mod base;
pub mod ext;

/// Describes a target which can be debugged by a
/// [`GdbStub`](struct.GdbStub.html).
///
/// TODO: discuss how modular approach works
pub trait Target {
    /// The target's architecture.
    type Arch: Arch;

    /// A target-specific **fatal** error.
    type Error;

    /// Base operations required to debug any target, such as stopping/resuming
    /// the target, reading/writing from memory/registers, etc....
    fn base_ops(&mut self) -> base::BaseOps<'_, Self::Arch, Self::Error>;

    /// Set/Remote software breakpoints.
    fn sw_breakpoint(&mut self) -> ext::SwBreakpointExt<Self>;

    /// Set/Remote hardware breakpoints.
    fn hw_breakpoint(&mut self) -> Option<ext::HwBreakpointExt<Self>> {
        None
    }

    /// Set/Remote hardware watchpoints.
    fn hw_watchpoint(&mut self) -> Option<ext::HwWatchpointExt<Self>> {
        None
    }

    /// Handle custom GDB `monitor` commands.
    fn monitor_cmd(&mut self) -> Option<ext::MonitorCmdExt<Self>> {
        None
    }
}
