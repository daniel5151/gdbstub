use core::fmt::{self, Display};
use core::marker::PhantomData;

use managed::ManagedSlice;

use crate::conn::Connection;
use crate::target::Target;

use super::core_impl::GdbStubImpl;
use super::GdbStub;

/// An error which may occur when building a [`GdbStub`].
#[derive(Debug)]
pub enum GdbStubBuilderError {
    /// Must provide buffer using `with_packet_buffer` in `#![no_std]` mode.
    MissingPacketBuffer,
    /// Custom packet buffer size is larger than the provided buffer's length.
    PacketBufSizeMismatch,
}

impl Display for GdbStubBuilderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use self::GdbStubBuilderError::*;
        match self {
            MissingPacketBuffer => write!(
                f,
                "Must provide buffer using `with_packet_buffer` in `#![no_std]` mode."
            ),
            PacketBufSizeMismatch => write!(
                f,
                "`packet_buffer_size` is larger than `with_packet_buffer`'s size."
            ),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for GdbStubBuilderError {}

/// Helper to construct and customize [`GdbStub`].
pub struct GdbStubBuilder<'a, T: Target, C: Connection> {
    conn: C,
    packet_buffer: Option<&'a mut [u8]>,
    packet_buffer_size: Option<usize>,

    _target: PhantomData<T>,
}

impl<'a, T: Target, C: Connection> GdbStubBuilder<'a, T, C> {
    /// Create a new `GdbStubBuilder` using the provided Connection.
    pub fn new(conn: C) -> GdbStubBuilder<'static, T, C> {
        GdbStubBuilder {
            conn,
            packet_buffer: None,
            packet_buffer_size: None,

            _target: PhantomData,
        }
    }

    /// Use a pre-allocated packet buffer (instead of heap-allocating).
    ///
    /// _Note:_ This method is _required_ when the `alloc` feature is disabled!
    pub fn with_packet_buffer(mut self, packet_buffer: &'a mut [u8]) -> Self {
        self.packet_buffer = Some(packet_buffer);
        self
    }

    /// Specify a custom size for the packet buffer. Defaults to 4096 bytes.
    ///
    /// When used alongside `with_packet_buffer`, the provided `size` must be
    /// less than or equal to the length of the packet buffer.
    pub fn packet_buffer_size(mut self, size: usize) -> Self {
        self.packet_buffer_size = Some(size);
        self
    }

    /// Build the GdbStub, returning an error if something went wrong.
    pub fn build(self) -> Result<GdbStub<'a, T, C>, GdbStubBuilderError> {
        let packet_buffer = match self.packet_buffer {
            Some(buf) => {
                let buf = match self.packet_buffer_size {
                    Some(custom_len) => {
                        if custom_len > buf.len() {
                            return Err(GdbStubBuilderError::PacketBufSizeMismatch);
                        } else {
                            &mut buf[..custom_len]
                        }
                    }
                    None => buf,
                };
                ManagedSlice::Borrowed(buf)
            }
            None => {
                cfg_if::cfg_if! {
                    if #[cfg(feature = "alloc")] {
                        use alloc::vec;
                        // need to pick some arbitrary value to report to GDB
                        // 4096 seems reasonable?
                        let len = self.packet_buffer_size.unwrap_or(4096);
                        ManagedSlice::Owned(vec![0; len])
                    } else {
                        return Err(GdbStubBuilderError::MissingPacketBuffer);
                    }
                }
            }
        };

        Ok(GdbStub {
            conn: self.conn,
            packet_buffer,
            inner: GdbStubImpl::new(),
        })
    }
}
