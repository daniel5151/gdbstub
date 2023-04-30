use crate::conn::Connection;
use crate::conn::ConnectionExt;
use alloc::boxed::Box;

impl<E> Connection for Box<dyn Connection<Error = E>> {
    type Error = E;

    fn write(&mut self, byte: u8) -> Result<(), Self::Error> {
        (**self).write(byte)
    }

    fn write_all(&mut self, buf: &[u8]) -> Result<(), Self::Error> {
        (**self).write_all(buf)
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        (**self).flush()
    }

    fn on_session_start(&mut self) -> Result<(), Self::Error> {
        (**self).on_session_start()
    }
}

impl<E> Connection for Box<dyn ConnectionExt<Error = E>> {
    type Error = E;

    fn write(&mut self, byte: u8) -> Result<(), Self::Error> {
        (**self).write(byte)
    }

    fn write_all(&mut self, buf: &[u8]) -> Result<(), Self::Error> {
        (**self).write_all(buf)
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        (**self).flush()
    }

    fn on_session_start(&mut self) -> Result<(), Self::Error> {
        (**self).on_session_start()
    }
}

impl<E> ConnectionExt for Box<dyn ConnectionExt<Error = E>> {
    fn read(&mut self) -> Result<u8, Self::Error> {
        (**self).read()
    }

    fn peek(&mut self) -> Result<Option<u8>, Self::Error> {
        (**self).peek()
    }
}
