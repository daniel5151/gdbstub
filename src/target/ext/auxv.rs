//! Access the target’s auxiliary vector.
use crate::target::Target;
use crate::target::TargetResult;

/// Target Extension - Access the target’s auxiliary vector.
pub trait Auxv: Target {
    /// Get auxiliary vector from the target.
    ///
    /// Return the number of bytes written into `buf` (which may be less than
    /// `length`).
    ///
    /// If `offset` is greater than the length of the underlying data, return
    /// `Ok(0)`.
    fn get_auxv(&self, offset: u64, length: usize, buf: &mut [u8]) -> TargetResult<usize, Self>;
}

define_ext!(AuxvOps, Auxv);
