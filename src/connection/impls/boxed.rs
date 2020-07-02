use crate::Connection;

use alloc::boxed::Box;

impl<E> Connection for Box<dyn Connection<Error = E>> {
    type Error = E;

    fn read(&mut self) -> Result<u8, Self::Error> {
        (**self).read()
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), Self::Error> {
        (**self).read_exact(buf)
    }

    fn write(&mut self, byte: u8) -> Result<(), Self::Error> {
        (**self).write(byte)
    }

    fn peek(&mut self) -> Result<Option<u8>, Self::Error> {
        (**self).peek()
    }
}
