use std::io;
use std::os::unix::net::UnixStream;

use crate::conn::Connection;
use crate::conn::ConnectionExt;

// TODO: Remove PeekExt once rust-lang/rust#73761 is stabilized
trait PeekExt {
    fn peek(&self, buf: &mut [u8]) -> io::Result<usize>;
}

impl PeekExt for UnixStream {
    #[cfg(feature = "paranoid_unsafe")]
    #[allow(clippy::panic)]
    fn peek(&self, _buf: &mut [u8]) -> io::Result<usize> {
        panic!("cannot use `UnixStream::peek` with `paranoid_unsafe` until rust-lang/rust#73761 is stabilized");
    }

    #[cfg(not(feature = "paranoid_unsafe"))]
    #[allow(non_camel_case_types)]
    fn peek(&self, buf: &mut [u8]) -> io::Result<usize> {
        use core::ffi::c_void;
        use std::os::unix::io::AsRawFd;

        // Define some libc types inline (to avoid bringing in entire libc dep)

        // every platform supported by the libc crate uses c_int = i32
        type c_int = i32;
        type size_t = usize;
        type ssize_t = isize;
        const MSG_PEEK: c_int = 2;
        extern "C" {
            fn recv(socket: c_int, buf: *mut c_void, len: size_t, flags: c_int) -> ssize_t;
        }

        // from std/sys/unix/mod.rs
        pub fn cvt(t: isize) -> io::Result<isize> {
            if t == -1 {
                Err(io::Error::last_os_error())
            } else {
                Ok(t)
            }
        }

        // from std/sys/unix/net.rs
        let ret = cvt(unsafe {
            recv(
                self.as_raw_fd(),
                buf.as_mut_ptr() as *mut c_void,
                buf.len(),
                MSG_PEEK,
            )
        })?;
        Ok(ret as usize)
    }
}

impl Connection for UnixStream {
    type Error = std::io::Error;

    fn write(&mut self, byte: u8) -> Result<(), Self::Error> {
        use std::io::Write;

        Write::write_all(self, &[byte])
    }

    fn write_all(&mut self, buf: &[u8]) -> Result<(), Self::Error> {
        use std::io::Write;

        Write::write_all(self, buf)
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        use std::io::Write;

        Write::flush(self)
    }
}

impl ConnectionExt for UnixStream {
    fn read(&mut self) -> Result<u8, Self::Error> {
        use std::io::Read;

        self.set_nonblocking(false)?;

        let mut buf = [0u8];
        match Read::read_exact(self, &mut buf) {
            Ok(_) => Ok(buf[0]),
            Err(e) => Err(e),
        }
    }

    fn peek(&mut self) -> Result<Option<u8>, Self::Error> {
        self.set_nonblocking(true)?;

        let mut buf = [0u8];
        match PeekExt::peek(self, &mut buf) {
            Ok(_) => Ok(Some(buf[0])),
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(None),
            Err(e) => Err(e),
        }
    }
}
