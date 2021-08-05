//! Provide Host I/O operations for the target.
use crate::arch::Arch;
use crate::target::{Target, TargetResult};
use bitflags::bitflags;

bitflags! {
    /// Host flags for opening files.
    /// [Open Flags]: https://sourceware.org/gdb/onlinedocs/gdb/Open-Flags.html
    pub struct HostOpenFlags: u32 {
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
    /// [mode_t Values]: https://sourceware.org/gdb/onlinedocs/gdb/mode_005ft-Values.html
    pub struct HostMode: u32 {
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

/// An interface to send pread data back to the GDB client.
pub struct HostIoOutput<'a> {
    cb: &'a mut dyn FnMut(&[u8]),
}

impl<'a> HostIoOutput<'a> {
    pub(crate) fn new(cb: &'a mut dyn FnMut(&[u8])) -> Self {
        Self { cb }
    }

    /// Write out raw file bytes to the GDB debugger.
    pub fn write(self, buf: &[u8]) {
        (self.cb)(buf)
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
    /// Close the open file corresponding to fd and return 0, or -1 if an error
    /// occurs.
    fn open(
        &mut self,
        filename: &[u8],
        flags: HostOpenFlags,
        mode: HostMode,
    ) -> TargetResult<i32, Self>;
}

define_ext!(HostIoOpenOps, HostIoOpen);

/// Nested Target Extension - Host I/O close operation.
pub trait HostIoClose: HostIo {
    /// Close the open file corresponding to fd and return 0, or -1 if an error
    /// occurs.
    fn close(&mut self, fd: i32) -> TargetResult<i64, Self>;
}

define_ext!(HostIoCloseOps, HostIoClose);

/// Nested Target Extension - Host I/O pread operation.
pub trait HostIoPread: HostIo {
    /// Read data from the open file corresponding to fd. Up to count bytes will
    /// be read from the file, starting at offset relative to the start of the
    /// file. The target may read fewer bytes; common reasons include packet
    /// size limits and an end-of-file condition. The number of bytes read is
    /// returned. Zero should only be returned for a successful read at the end
    /// of the file, or if count was zero.
    fn pread(
        &mut self,
        fd: i32,
        count: <Self::Arch as Arch>::Usize,
        offset: <Self::Arch as Arch>::Usize,
        output: HostIoOutput<'_>,
    ) -> TargetResult<(), Self>;
}

define_ext!(HostIoPreadOps, HostIoPread);

/// Nested Target Extension - Host I/O pwrite operation.
pub trait HostIoPwrite: HostIo {
    /// Write data (a binary buffer) to the open file corresponding to fd. Start
    /// the write at offset from the start of the file. Unlike many write system
    /// calls, there is no separate count argument; the length of data in the
    /// packet is used. ‘vFile:pwrite’ returns the number of bytes written,
    /// which may be shorter than the length of data, or -1 if an error
    /// occurred.
    fn pwrite(
        &mut self,
        fd: i32,
        offset: <Self::Arch as Arch>::Usize,
        data: &[u8],
    ) -> TargetResult<i32, Self>;
}

define_ext!(HostIoPwriteOps, HostIoPwrite);

/// Nested Target Extension - Host I/O fstat operation.
pub trait HostIoFstat: HostIo {
    /// Get information about the open file corresponding to fd. On success the
    /// information is returned as a binary attachment and the return value is
    /// the size of this attachment in bytes. If an error occurs the return
    /// value is -1.
    fn fstat(&mut self, fd: i32, output: HostIoOutput<'_>) -> TargetResult<i32, Self>;
}

define_ext!(HostIoFstatOps, HostIoFstat);

/// Nested Target Extension - Host I/O unlink operation.
pub trait HostIoUnlink: HostIo {
    /// Delete the file at filename on the target. Return 0, or -1 if an error
    /// occurs. The filename is a string.
    fn unlink(&mut self, filename: &[u8]) -> TargetResult<i32, Self>;
}

define_ext!(HostIoUnlinkOps, HostIoUnlink);

/// Nested Target Extension - Host I/O readlink operation.
pub trait HostIoReadlink: HostIo {
    /// Read value of symbolic link filename on the target. Return the number of
    /// bytes read, or -1 if an error occurs.

    /// The data read should be returned as a binary attachment on success. If
    /// zero bytes were read, the response should include an empty binary
    /// attachment (i.e. a trailing semicolon). The return value is the number
    /// of target bytes read; the binary attachment may be longer if some
    /// characters were escaped.
    fn readlink(&mut self, filename: &[u8]) -> TargetResult<i32, Self>;
}

define_ext!(HostIoReadlinkOps, HostIoReadlink);

/// Nested Target Extension - Host I/O setfs operation.
pub trait HostIoSetfs: HostIo {
    /// Select the filesystem on which vFile operations with filename arguments
    /// will operate. This is required for GDB to be able to access files on
    /// remote targets where the remote stub does not share a common filesystem
    /// with the inferior(s).

    /// If pid is nonzero, select the filesystem as seen by process pid. If pid
    /// is zero, select the filesystem as seen by the remote stub. Return 0 on
    /// success, or -1 if an error occurs. If vFile:setfs: indicates success,
    /// the selected filesystem remains selected until the next successful
    /// vFile:setfs: operation.
    fn setfs(&mut self, pid: usize) -> TargetResult<i32, Self>;
}

define_ext!(HostIoSetfsOps, HostIoSetfs);
