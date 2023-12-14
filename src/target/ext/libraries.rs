//! TODO

use crate::target::Target;
use crate::target::TargetResult;

/// Target Extension - List a target's libraries.
pub trait Libraries: Target {
    /// Get library list XML for this target.
    ///
    /// See the [GDB Documentation] for a description of the format.
    ///
    /// [GDB Documentation]: https://sourceware.org/gdb/current/onlinedocs/gdb.html/Library-List-Format.html
    ///
    /// Return the number of bytes written into `buf` (which may be less than
    /// `length`).
    ///
    /// If `offset` is greater than the length of the underlying data, return
    /// `Ok(0)`.
    fn get_libraries(
        &self,
        offset: u64,
        length: usize,
        buf: &mut [u8],
    ) -> TargetResult<usize, Self>;
}

define_ext!(LibrariesOps, Libraries);

/// Target Extension - List an SVR4 (System-V/Unix) target's libraries.
pub trait LibrariesSvr4: Target {
    /// Get library list XML for this target.
    ///
    /// See the [GDB Documentation] for a description of the format.
    ///
    /// [GDB Documentation]: https://sourceware.org/gdb/current/onlinedocs/gdb.html/Library-List-Format-for-SVR4-Targets.html
    ///
    /// Return the number of bytes written into `buf` (which may be less than
    /// `length`).
    ///
    /// If `offset` is greater than the length of the underlying data, return
    /// `Ok(0)`.
    fn get_libraries_svr4(
        &self,
        offset: u64,
        length: usize,
        buf: &mut [u8],
    ) -> TargetResult<usize, Self>;
}

define_ext!(LibrariesSvr4Ops, LibrariesSvr4);
