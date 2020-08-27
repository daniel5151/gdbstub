//! Target Extension traits which can be implemented to support additional GDB
//! debugging features.
//!
//! If there's a GDB feature that you need that isn't implemented yet, feel free
//! to open an issue / file a PR on Github!

use crate::target::Target;

pub mod breakpoint;
pub mod monitor;
pub mod offsets;

// This could probably be expressed much more cleanly using trait aliases, but
// those aren't stable, so this'll have to do for now.
macro_rules! define_ext {
    ($extname:ident, $($exttrait:tt)+) => {
        #[allow(missing_docs)]
        pub type $extname<'a, T> =
            &'a mut dyn $($exttrait)+<Arch = <T as Target>::Arch, Error = <T as Target>::Error>;
    };
}

define_ext!(SwBreakpointOps, breakpoint::SwBreakpoint);
define_ext!(HwBreakpointOps, breakpoint::HwBreakpoint);
define_ext!(HwWatchpointOps, breakpoint::HwWatchpoint);
define_ext!(MonitorCmdOps, monitor::MonitorCmd);
define_ext!(SectionOffsetsOps, offsets::OffsetsCmd);
