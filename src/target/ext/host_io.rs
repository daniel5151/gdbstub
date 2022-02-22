//! Provide Host I/O operations for the target.
use bitflags::bitflags;

use crate::arch::Arch;
use crate::target::Target;

bitflags! {
    /// Host flags for opening files.
    ///
    /// Extracted from the GDB documentation at
    /// [Open Flags](https://sourceware.org/gdb/current/onlinedocs/gdb/Open-Flags.html#Open-Flags),
    /// and the LLDB source code at
    /// [`lldb/include/lldb/Host/File.h`](https://github.com/llvm/llvm-project/blob/ec642ceebc1aacc8b16249df7734b8cf90ae2963/lldb/include/lldb/Host/File.h#L47-L66)
    pub struct HostIoOpenFlags: u32 {
        /// A read-only file.
        const O_RDONLY = 0x0;
        /// A write-only file.
        const O_WRONLY = 0x1;
        /// A read-write file.
        const O_RDWR = 0x2;
        /// Append to an existing file.
        const O_APPEND = 0x8;
        /// Create a non-existent file.
        const O_CREAT = 0x200;
        /// Truncate an existing file.
        const O_TRUNC = 0x400;
        /// Exclusive access.
        const O_EXCL = 0x800;

        /// LLDB extension: Do not block.
        const O_NONBLOCK = 1 << 28;
        /// LLDB extension: Do not follow symlinks.
        const O_DONT_FOLLOW_SYMLINKS = 1 << 29;
        /// LLDB extension: Close the file when executing a new process.
        const O_CLOSE_ON_EXEC = 1 << 30;
        /// LLDB extension: Invalid value.
        const O_INVALID = 1 << 31;
    }
}

bitflags! {
    /// Host file permissions.
    ///
    /// Extracted from the GDB documentation at
    /// [mode_t Values](https://sourceware.org/gdb/current/onlinedocs/gdb/mode_005ft-Values.html#mode_005ft-Values)
    pub struct HostIoOpenMode: u32 {
        /// A regular file.
        const S_IFREG = 0o100000;
        /// A directory.
        const S_IFDIR = 0o40000;
        /// User read permissions.
        const S_IRUSR = 0o400;
        /// User write permissions.
        const S_IWUSR = 0o200;
        /// User execute permissions.
        const S_IXUSR = 0o100;
        /// Group read permissions.
        const S_IRGRP = 0o40;
        /// Group write permissions
        const S_IWGRP = 0o20;
        /// Group execute permissions.
        const S_IXGRP = 0o10;
        /// World read permissions.
        const S_IROTH = 0o4;
        /// World write permissions
        const S_IWOTH = 0o2;
        /// World execute permissions.
        const S_IXOTH = 0o1;
    }
}

/// Data returned by a host fstat request.
///
/// Extracted from the GDB documentation at
/// [struct stat](https://sourceware.org/gdb/current/onlinedocs/gdb/struct-stat.html#struct-stat)
#[derive(Debug)]
pub struct HostIoStat {
    /// The device.
    pub st_dev: u32,
    /// The inode.
    pub st_ino: u32,
    /// Protection bits.
    pub st_mode: HostIoOpenMode,
    /// The number of hard links.
    pub st_nlink: u32,
    /// The user id of the owner.
    pub st_uid: u32,
    /// The group id of the owner.
    pub st_gid: u32,
    /// The device type, if an inode device.
    pub st_rdev: u32,
    /// The size of the file in bytes.
    pub st_size: u64,
    /// The blocksize for the filesystem.
    pub st_blksize: u64,
    /// The number of blocks allocated.
    pub st_blocks: u64,
    /// The last time the file was accessed, in seconds since the epoch.
    pub st_atime: u32,
    /// The last time the file was modified, in seconds since the epoch.
    pub st_mtime: u32,
    /// The last time the file was changed, in seconds since the epoch.
    pub st_ctime: u32,
}

/// Select the filesystem vFile operations will operate on. Used by vFile setfs
/// command.
#[derive(Debug)]
pub enum FsKind {
    /// Select the filesystem as seen by the remote stub.
    Stub,
    /// Select the filesystem as seen by process pid.
    Pid(crate::common::Pid),
}

/// Errno values for Host I/O operations.
///
/// Extracted from the GDB documentation at
/// <https://sourceware.org/gdb/onlinedocs/gdb/Errno-Values.html>
#[derive(Debug)]
pub enum HostIoErrno {
    /// Operation not permitted (POSIX.1-2001).
    EPERM = 1,
    /// No such file or directory (POSIX.1-2001).
    ///
    /// Typically, this error results when a specified pathname does not exist,
    /// or one of the components in the directory prefix of a pathname does not
    /// exist, or the specified pathname is a dangling symbolic link.
    ENOENT = 2,
    /// Interrupted function call (POSIX.1-2001); see signal(7).
    EINTR = 4,
    /// Bad file descriptor (POSIX.1-2001).
    EBADF = 9,
    /// Permission denied (POSIX.1-2001).
    EACCES = 13,
    /// Bad address (POSIX.1-2001).
    EFAULT = 14,
    /// Device or resource busy (POSIX.1-2001).
    EBUSY = 16,
    /// File exists (POSIX.1-2001).
    EEXIST = 17,
    /// No such device (POSIX.1-2001).
    ENODEV = 19,
    /// Not a directory (POSIX.1-2001).
    ENOTDIR = 20,
    /// Is a directory (POSIX.1-2001).
    EISDIR = 21,
    /// Invalid argument (POSIX.1-2001).
    EINVAL = 22,
    /// Too many open files in system (POSIX.1-2001). On Linux, this is probably
    /// a result of encountering the /proc/sys/fs/file-max limit (see proc(5)).
    ENFILE = 23,
    /// Too many open files (POSIX.1-2001). Commonly caused by exceeding the
    /// RLIMIT_NOFILE resource limit described in getrlimit(2).
    EMFILE = 24,
    /// File too large (POSIX.1-2001).
    EFBIG = 27,
    /// No space left on device (POSIX.1-2001).
    ENOSPC = 28,
    /// Invalid seek (POSIX.1-2001).
    ESPIPE = 29,
    /// Read-only filesystem (POSIX.1-2001).
    EROFS = 30,
    /// Filename too long (POSIX.1-2001).
    ENAMETOOLONG = 91,
    /// Unknown errno - there may not be a GDB mapping for this value
    EUNKNOWN = 9999,
}

/// The error type for Host I/O operations.
pub enum HostIoError<E> {
    /// An operation-specific non-fatal error code.
    ///
    /// See [`HostIoErrno`] for more details.
    Errno(HostIoErrno),
    /// A target-specific fatal error.
    ///
    /// **WARNING:** Returning this error will immediately halt the target's
    /// execution and return a `GdbStubError::TargetError`!
    ///
    /// Note that returning this error will _not_ notify the GDB client that the
    /// debugging session has been terminated, making it possible to resume
    /// execution after resolving the error and/or setting up a post-mortem
    /// debugging environment.
    Fatal(E),
}

/// When the `std` feature is enabled, `HostIoError` implements
/// `From<std::io::Error>`, mapping [`std::io::ErrorKind`] to the appropriate
/// [`HostIoErrno`] when possible, and falling back to [`HostIoErrno::EUNKNOWN`]
/// when no mapping exists.
#[cfg(feature = "std")]
impl<E> From<std::io::Error> for HostIoError<E> {
    fn from(e: std::io::Error) -> HostIoError<E> {
        use std::io::ErrorKind::*;
        let errno = match e.kind() {
            PermissionDenied => HostIoErrno::EPERM,
            NotFound => HostIoErrno::ENOENT,
            Interrupted => HostIoErrno::EINTR,
            AlreadyExists => HostIoErrno::EEXIST,
            InvalidInput => HostIoErrno::EINVAL,
            _ => HostIoErrno::EUNKNOWN,
        };
        HostIoError::Errno(errno)
    }
}

/// A specialized `Result` type for Host I/O operations. Supports reporting
/// non-fatal errors back to the GDB client.
///
/// See [`HostIoError`] for more details.
pub type HostIoResult<T, Tgt> = Result<T, HostIoError<<Tgt as Target>::Error>>;

/// Target Extension - Perform I/O operations on host
pub trait HostIo: Target {
    /// Support `open` operation.
    #[inline(always)]
    fn support_open(&mut self) -> Option<HostIoOpenOps<'_, Self>> {
        None
    }

    /// Support `close` operation.
    #[inline(always)]
    fn support_close(&mut self) -> Option<HostIoCloseOps<'_, Self>> {
        None
    }

    /// Support `pread` operation.
    #[inline(always)]
    fn support_pread(&mut self) -> Option<HostIoPreadOps<'_, Self>> {
        None
    }

    /// Support `pwrite` operation.
    #[inline(always)]
    fn support_pwrite(&mut self) -> Option<HostIoPwriteOps<'_, Self>> {
        None
    }

    /// Support `fstat` operation.
    #[inline(always)]
    fn support_fstat(&mut self) -> Option<HostIoFstatOps<'_, Self>> {
        None
    }

    /// Support `unlink` operation.
    #[inline(always)]
    fn support_unlink(&mut self) -> Option<HostIoUnlinkOps<'_, Self>> {
        None
    }

    /// Support `readlink` operation.
    #[inline(always)]
    fn support_readlink(&mut self) -> Option<HostIoReadlinkOps<'_, Self>> {
        None
    }

    /// Support `setfs` operation.
    #[inline(always)]
    fn support_setfs(&mut self) -> Option<HostIoSetfsOps<'_, Self>> {
        None
    }
}

define_ext!(HostIoOps, HostIo);

/// Nested Target Extension - Host I/O open operation.
pub trait HostIoOpen: HostIo {
    /// Open a file at `filename` and return a file descriptor for it, or return
    /// [`HostIoError::Errno`] if an error occurs.
    ///
    /// `flags` are the flags used when opening the file (see
    /// [`HostIoOpenFlags`]), and `mode` is the mode used if the file is
    /// created (see [`HostIoOpenMode`]).
    fn open(
        &mut self,
        filename: &[u8],
        flags: HostIoOpenFlags,
        mode: HostIoOpenMode,
    ) -> HostIoResult<u32, Self>;
}

define_ext!(HostIoOpenOps, HostIoOpen);

/// Nested Target Extension - Host I/O close operation.
pub trait HostIoClose: HostIo {
    /// Close the open file corresponding to `fd`.
    fn close(&mut self, fd: u32) -> HostIoResult<(), Self>;
}

define_ext!(HostIoCloseOps, HostIoClose);

/// Nested Target Extension - Host I/O pread operation.
pub trait HostIoPread: HostIo {
    /// Read data from the open file corresponding to `fd`.
    ///
    /// Up to `count` bytes will be read from the file, starting at `offset`
    /// relative to the start of the file.
    ///
    /// Return the number of bytes written into `buf` (which may be less than
    /// `count`).
    ///
    /// If `offset` is greater than the length of the underlying data, return
    /// `Ok(0)`.
    fn pread(
        &mut self,
        fd: u32,
        count: usize,
        offset: u64,
        buf: &mut [u8],
    ) -> HostIoResult<usize, Self>;
}

define_ext!(HostIoPreadOps, HostIoPread);

/// Nested Target Extension - Host I/O pwrite operation.
pub trait HostIoPwrite: HostIo {
    /// Write `data` to the open file corresponding to `fd`.
    ///
    /// Start the write at `offset` from the start of the file.
    ///
    /// Return the number of bytes written, which may be shorter
    /// than the length of data, or [`HostIoError::Errno`] if an error occurred.
    fn pwrite(
        &mut self,
        fd: u32,
        offset: <Self::Arch as Arch>::Usize,
        data: &[u8],
    ) -> HostIoResult<<Self::Arch as Arch>::Usize, Self>;
}

define_ext!(HostIoPwriteOps, HostIoPwrite);

/// Nested Target Extension - Host I/O fstat operation.
pub trait HostIoFstat: HostIo {
    /// Get information about the open file corresponding to `fd`.
    ///
    /// On success return a [`HostIoStat`] struct.
    /// Return [`HostIoError::Errno`] if an error occurs.
    fn fstat(&mut self, fd: u32) -> HostIoResult<HostIoStat, Self>;
}

define_ext!(HostIoFstatOps, HostIoFstat);

/// Nested Target Extension - Host I/O unlink operation.
pub trait HostIoUnlink: HostIo {
    /// Delete the file at `filename` on the target.
    fn unlink(&mut self, filename: &[u8]) -> HostIoResult<(), Self>;
}

define_ext!(HostIoUnlinkOps, HostIoUnlink);

/// Nested Target Extension - Host I/O readlink operation.
pub trait HostIoReadlink: HostIo {
    /// Read value of symbolic link `filename` on the target.
    ///
    /// Return the number of bytes written into `buf`.
    ///
    /// Unlike most other Host IO handlers, if the resolved file path exceeds
    /// the length of the provided `buf`, the target should NOT return a
    /// partial response, and MUST return a `Err(HostIoErrno::ENAMETOOLONG)`.
    fn readlink(&mut self, filename: &[u8], buf: &mut [u8]) -> HostIoResult<usize, Self>;
}

define_ext!(HostIoReadlinkOps, HostIoReadlink);

/// Nested Target Extension - Host I/O setfs operation.
pub trait HostIoSetfs: HostIo {
    /// Select the filesystem on which vFile operations with filename arguments
    /// will operate. This is required for GDB to be able to access files on
    /// remote targets where the remote stub does not share a common filesystem
    /// with the inferior(s).
    ///
    /// See [`FsKind`] for the meaning of `fs`.
    ///
    /// If setfs indicates success, the selected filesystem remains selected
    /// until the next successful setfs operation.
    fn setfs(&mut self, fs: FsKind) -> HostIoResult<(), Self>;
}

define_ext!(HostIoSetfsOps, HostIoSetfs);
