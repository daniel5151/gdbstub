//! Target Extension traits which can optionally be implemented to enable
//! additional GDB debugging features.

use crate::target::Target;

pub mod breakpoint;
pub mod monitor;

// This could probably be expressed much more cleanly using trait aliases, but
// those aren't stable, so this'll have to do for now.
macro_rules! define_ext {
    ($extname:ident, $($exttrait:tt)+) => {
        #[allow(missing_docs)]
        pub type $extname<'a, T> =
            &'a mut dyn $($exttrait)+<Arch = <T as Target>::Arch, Error = <T as Target>::Error>;
    };
}

define_ext!(SwBreakpointExt, breakpoint::SwBreakpoint);
define_ext!(HwBreakpointExt, breakpoint::HwBreakpoint);
define_ext!(HwWatchpointExt, breakpoint::HwWatchpoint);
define_ext!(MonitorCmdExt, monitor::MonitorCmd);
