//! Provide Host I/O operations for the target.
use crate::arch::Arch;
use crate::target::Target;
use bitflags::bitflags;

bitflags! {
    /// Host flags for opening files.
    ///
    /// Extracted from the GDB documentation at
    /// [Open Flags](https://sourceware.org/gdb/current/onlinedocs/gdb/Open-Flags.html#Open-Flags)
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
    }
}

bitflags! {
    /// Host file permissions.
    ///
    /// Extracted from the GDB documentation at
    /// [mode_t Values](https://sourceware.org/gdb/current/onlinedocs/gdb/mode_005ft-Values.html#mode_005ft-Values)
    pub struct HostIoMode: u32 {
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
pub struct HostStat {
    /// The device.
    pub st_dev: u32,
    /// The inode.
    pub st_ino: u32,
    /// Protection bits.
    pub st_mode: HostIoMode,
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
    /// select the filesystem as seen by the remote stub
    Stub,
    /// select the filesystem as seen by process pid
    Pid(crate::common::Pid),
}

/// Errno values for Host I/O operations.
///
/// Extracted from the GDB documentation at
/// [Errno Values]: https://sourceware.org/gdb/onlinedocs/gdb/Errno-Values.html
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
    /// execution and return a `GdbStubError::TargetError` from `GdbStub::run`!
    ///
    /// Note that the debugging session will will _not_ be terminated, and can
    /// be resumed by calling `GdbStub::run` after resolving the error and/or
    /// setting up a post-mortem debugging environment.
    Fatal(E),
}

/// A specialized `Result` type for Host I/O operations. Supports reporting
/// non-fatal errors back to the GDB client.
///
/// See [`HostIoError`] for more details.
pub type HostIoResult<T, Tgt> = Result<T, HostIoError<<Tgt as Target>::Error>>;

/// Zero-sized type token that ensures PreadOutput::write is called
pub struct PreadToken<'a>(core::marker::PhantomData<&'a *mut ()>);

/// An interface to send pread data back to the GDB client.
pub struct PreadOutput<'a> {
    cb: &'a mut dyn FnMut(&[u8]),
    token: PreadToken<'a>,
}

impl<'a> PreadOutput<'a> {
    pub(crate) fn new(cb: &'a mut dyn FnMut(&[u8])) -> Self {
        Self {
            cb,
            token: PreadToken(core::marker::PhantomData),
        }
    }

    /// Write out raw file bytes to the GDB debugger.
    pub fn write(self, buf: &[u8]) -> PreadToken<'a> {
        (self.cb)(buf);
        self.token
    }
}

/// Target Extension - Perform I/O operations on host
pub trait HostIo: Target {
    /// Enable open operation.
    #[inline(always)]
    fn enable_open(&mut self) -> Option<HostIoOpenOps<Self>> {
        None
    }
    /// Enable close operation.
    #[inline(always)]
    fn enable_close(&mut self) -> Option<HostIoCloseOps<Self>> {
        None
    }
    /// Enable pread operation.
    #[inline(always)]
    fn enable_pread(&mut self) -> Option<HostIoPreadOps<Self>> {
        None
    }
    /// Enable pwrite operation.
    #[inline(always)]
    fn enable_pwrite(&mut self) -> Option<HostIoPwriteOps<Self>> {
        None
    }
    /// Enable fstat operation.
    #[inline(always)]
    fn enable_fstat(&mut self) -> Option<HostIoFstatOps<Self>> {
        None
    }
    /// Enable unlink operation.
    #[inline(always)]
    fn enable_unlink(&mut self) -> Option<HostIoUnlinkOps<Self>> {
        None
    }
    /// Enable readlink operation.
    #[inline(always)]
    fn enable_readlink(&mut self) -> Option<HostIoReadlinkOps<Self>> {
        None
    }
    /// Enable setfs operation.
    #[inline(always)]
    fn enable_setfs(&mut self) -> Option<HostIoSetfsOps<Self>> {
        None
    }
}

define_ext!(HostIoOps, HostIo);

/// Nested Target Extension - Host I/O open operation.
pub trait HostIoOpen: HostIo {
    /// Open a file at filename and return a file descriptor for it, or return
    /// -1 if an error occurs.
    ///
    /// The filename is a string, flags is an integer indicating a mask of open
    /// flags (see [`HostIoOpenFlags`]), and mode is an integer indicating a
    /// mask of mode bits to use if the file is created (see
    /// [`HostIoMode`]).
    fn open(
        &mut self,
        filename: &[u8],
        flags: HostIoOpenFlags,
        mode: HostIoMode,
    ) -> HostIoResult<u32, Self>;
}

define_ext!(HostIoOpenOps, HostIoOpen);

/// Nested Target Extension - Host I/O close operation.
pub trait HostIoClose: HostIo {
    /// Close the open file corresponding to fd and return 0, or -1 if an error
    /// occurs.
    fn close(&mut self, fd: i32) -> HostIoResult<u32, Self>;
}

define_ext!(HostIoCloseOps, HostIoClose);

/// Nested Target Extension - Host I/O pread operation.
pub trait HostIoPread: HostIo {
    /// Read data from the open file corresponding to fd.
    ///
    /// Up to count bytes will be read from the file, starting at offset
    /// relative to the start of the file.
    ///
    /// The target may read fewer bytes; common reasons include packet size
    /// limits and an end-of-file condition.
    ///
    /// The number of bytes read is returned. Zero should only be returned for a
    /// successful read at the end of the file, or if count was zero.
    fn pread<'a>(
        &mut self,
        fd: i32,
        count: <Self::Arch as Arch>::Usize,
        offset: <Self::Arch as Arch>::Usize,
        output: PreadOutput<'a>,
    ) -> HostIoResult<PreadToken<'a>, Self>;
}

define_ext!(HostIoPreadOps, HostIoPread);

/// Nested Target Extension - Host I/O pwrite operation.
pub trait HostIoPwrite: HostIo {
    /// Write data (a binary buffer) to the open file corresponding to fd.
    ///
    /// Start the write at offset from the start of the file.
    ///
    /// Unlike many write system calls, there is no separate count argument; the
    /// length of data in the packet is used.
    ///
    /// ‘vFile:pwrite’ returns the number of bytes written, which may be shorter
    /// than the length of data, or -1 if an error occurred.
    fn pwrite(
        &mut self,
        fd: i32,
        offset: <Self::Arch as Arch>::Usize,
        data: &[u8],
    ) -> HostIoResult<u32, Self>;
}

define_ext!(HostIoPwriteOps, HostIoPwrite);

/// Nested Target Extension - Host I/O fstat operation.
pub trait HostIoFstat: HostIo {
    /// Get information about the open file corresponding to fd.
    ///
    /// On success the information is returned as a binary attachment and the
    /// return value is the size of this attachment in bytes. If an error occurs
    /// the return value is -1.
    fn fstat(&mut self, fd: i32) -> HostIoResult<HostStat, Self>;
}

define_ext!(HostIoFstatOps, HostIoFstat);

/// Nested Target Extension - Host I/O unlink operation.
pub trait HostIoUnlink: HostIo {
    /// Delete the file at filename on the target.
    ///
    /// Return 0, or -1 if an error occurs.
    fn unlink(&mut self, filename: &[u8]) -> HostIoResult<u32, Self>;
}

define_ext!(HostIoUnlinkOps, HostIoUnlink);

/// Nested Target Extension - Host I/O readlink operation.
pub trait HostIoReadlink: HostIo {
    /// Read value of symbolic link filename on the target. Return the number of
    /// bytes read, or -1 if an error occurs.
    ///
    /// The data read should be returned as a binary attachment on success. If
    /// zero bytes were read, the response should include an empty binary
    /// attachment (i.e. a trailing semicolon).
    ///
    /// The return value is the number of target bytes read; the binary
    /// attachment may be longer if some characters were escaped.
    fn readlink(&mut self, filename: &[u8]) -> HostIoResult<u32, Self>;
}

define_ext!(HostIoReadlinkOps, HostIoReadlink);

/// Nested Target Extension - Host I/O setfs operation.
pub trait HostIoSetfs: HostIo {
    /// Select the filesystem on which vFile operations with filename arguments
    /// will operate. This is required for GDB to be able to access files on
    /// remote targets where the remote stub does not share a common filesystem
    /// with the inferior(s).
    ///
    /// If pid is nonzero, select the filesystem as seen by process pid. If pid
    /// is zero, select the filesystem as seen by the remote stub.
    ///
    /// Return 0 on success, or -1 if an error occurs. If vFile:setfs: indicates
    /// success, the selected filesystem remains selected until the next
    /// successful vFile:setfs: operation.
    fn setfs(&mut self, fs: FsKind) -> HostIoResult<u32, Self>;
}

define_ext!(HostIoSetfsOps, HostIoSetfs);
