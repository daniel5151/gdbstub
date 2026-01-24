//! Report information about the loaded shared libraries for targets where there
//! are possibly multiple files to be debugged mapped into the same address
//! space.

use crate::target::Target;
use crate::target::TargetResult;

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

/// Target Extension - List a target's libraries (Windows/generic format).
///
/// This is used for targets where library offsets are maintained externally
/// (e.g., Windows PE targets). Unlike SVR4 format, this uses a simpler XML
/// structure with segment addresses.
pub trait Libraries: Target {
    /// Get library list XML for this target.
    ///
    /// The expected XML format is:
    /// ```xml
    /// <library-list version="1.0">
    /// <library name="C:\Windows\notepad.exe"><segment address="0x00401000"/></library>
    /// <library name="C:\Windows\SYSTEM32\ntdll.dll"><segment address="0x774d1000"/></library>
    /// <library name="C:\Windows\system32\kernel32.dll"><segment address="0x772b1000"/></library>
    /// <library name="C:\Windows\system32\KernelBase.dll"><segment address="0x7fefd701000"/></library>
    /// </library-list>
    /// ```
    ///
    /// Note that on Windows, the `address` is not the image base, but the
    /// address of the first section. See the [GDB Documentation] for more
    /// details.
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
