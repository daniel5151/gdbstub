//! Implementations of the [`Connection`] trait for various built-in types
// TODO: impl Connection for all `Read + Write` (blocked on specialization)

#[cfg(feature = "alloc")]
mod boxed;

#[cfg(feature = "std")]
mod tcpstream;

#[cfg(all(feature = "std", unix))]
mod unixstream;

use crate::Connection;
use crate::ConnectionExt;

impl<E> Connection for &mut dyn Connection<Error = E> {
    type Error = E;

    fn write(&mut self, byte: u8) -> Result<(), Self::Error> {
        (**self).write(byte)
    }

    fn write_all(&mut self, buf: &[u8]) -> Result<(), Self::Error> {
        (**self).write_all(buf)
    }

    fn peek(&mut self) -> Result<Option<u8>, Self::Error> {
        (**self).peek()
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        (**self).flush()
    }

    fn on_session_start(&mut self) -> Result<(), Self::Error> {
        (**self).on_session_start()
    }
}

impl<E> ConnectionExt for &mut dyn ConnectionExt<Error = E> {
    fn read(&mut self) -> Result<u8, Self::Error> {
        (**self).read()
    }
}

impl<E> Connection for &mut dyn ConnectionExt<Error = E> {
    type Error = E;

    fn write(&mut self, byte: u8) -> Result<(), Self::Error> {
        (**self).write(byte)
    }

    fn write_all(&mut self, buf: &[u8]) -> Result<(), Self::Error> {
        (**self).write_all(buf)
    }

    fn peek(&mut self) -> Result<Option<u8>, Self::Error> {
        (**self).peek()
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        (**self).flush()
    }

    fn on_session_start(&mut self) -> Result<(), Self::Error> {
        (**self).on_session_start()
    }
}
