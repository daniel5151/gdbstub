use gdbstub::conn::Connection;

pub struct TcpConnection {
    sock: i32,
    fd: i32,
}

impl TcpConnection {
    pub fn new_localhost(port: u16) -> Result<TcpConnection, &'static str> {
        unsafe {
            let sockaddr = libc::sockaddr_in {
                sin_family: libc::AF_INET as _,
                sin_port: port.to_be(),
                // 127.0.0.1
                sin_addr: libc::in_addr {
                    s_addr: 0x7f000001u32.to_be(),
                },
                sin_zero: [0; 8],
            };
            let socklen = core::mem::size_of::<libc::sockaddr_in>();

            let sock = libc::socket(libc::AF_INET, libc::SOCK_STREAM, 0);
            if sock == -1 {
                return Err("could not create listen socket");
            }

            if libc::bind(sock, &sockaddr as *const _ as _, socklen as u32) < 0 {
                return Err("could not bind socket");
            }

            if libc::listen(sock, 1) < 0 {
                return Err("could not open socket for listening");
            }

            let fd = libc::accept(sock, core::ptr::null_mut(), &mut 0);
            if fd < 0 {
                return Err("could not accept socket connection");
            }

            Ok(TcpConnection { sock, fd })
        }
    }

    pub fn read(&mut self) -> Result<u8, &'static str> {
        let mut buf = [0];
        let ret = unsafe { libc::read(self.fd, buf.as_mut_ptr() as _, 1) };
        if ret == -1 || ret != 1 {
            Err("socket read failed")
        } else {
            Ok(buf[0])
        }
    }

    #[allow(dead_code)]
    pub fn peek(&mut self) -> Result<Option<u8>, &'static str> {
        let mut buf = [0];
        let ret = unsafe {
            libc::recv(
                self.fd,
                buf.as_mut_ptr() as *mut _,
                buf.len(),
                libc::MSG_PEEK,
            )
        };
        if ret == -1 || ret != 1 {
            Err("socket peek failed")
        } else {
            Ok(Some(buf[0]))
        }
    }
}

impl Drop for TcpConnection {
    fn drop(&mut self) {
        unsafe {
            libc::close(self.fd);
            libc::close(self.sock);
        }
    }
}

impl Connection for TcpConnection {
    type Error = &'static str;

    fn write(&mut self, b: u8) -> Result<(), &'static str> {
        let buf = [b];
        let ret = unsafe { libc::write(self.fd, buf.as_ptr() as _, 1) };
        if ret == -1 || ret != 1 {
            Err("socket write failed")
        } else {
            Ok(())
        }
    }

    fn flush(&mut self) -> Result<(), &'static str> {
        // huh, apparently flushing isn't a "thing" for Tcp streams.
        // see https://doc.rust-lang.org/src/std/net/tcp.rs.html#592-609
        Ok(())
    }
}
