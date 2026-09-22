//! (LLDB extension) Provide SBPF-specific actions for the target.

use crate::common::Tid;
use crate::target::Target;

/// (LLDB extension) Target extension for SBPF runtime state.
pub trait Sbpf: Target {
    /// Get the SBPF call stack for a given thread.
    ///
    /// Call `next_frame` once for each frame, ordered from the active frame to
    /// the oldest caller. `pc` is the current PC for the active frame and the
    /// return PC for callers. `frame_pointer` is the value of SBPF register
    /// r10 for that frame.
    fn sbpf_call_stack(
        &self,
        tid: Tid,
        next_frame: &mut dyn FnMut(u64, u64),
    ) -> Result<(), Self::Error>;
}

define_ext!(SbpfOps, Sbpf);
