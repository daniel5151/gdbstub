use super::managed::ManagedSlice;

/// Error value indicating insufficient capacity.
#[derive(Debug, Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
pub struct CapacityError<Element>(pub Element);

/// Wraps a ManagedSlice in a vec-like interface.
///
/// TODO?: Upstream ManagedVec into the main `managed` crate?
pub struct ManagedVec<'a, 'b, T: 'a> {
    buf: &'b mut ManagedSlice<'a, T>,
    len: usize,
}

impl<'a, 'b, T> ManagedVec<'a, 'b, T> {
    pub fn new(buf: &'b mut ManagedSlice<'a, T>) -> Self {
        ManagedVec { buf, len: 0 }
    }

    pub fn clear(&mut self) {
        match &mut self.buf {
            ManagedSlice::Borrowed(_) => self.len = 0,
            #[cfg(feature = "alloc")]
            ManagedSlice::Owned(buf) => buf.clear(),
        }
    }

    pub fn push(&mut self, value: T) -> Result<(), CapacityError<T>> {
        match &mut self.buf {
            ManagedSlice::Borrowed(buf) => {
                if self.len < buf.len() {
                    buf[self.len] = value;
                    self.len += 1;
                    Ok(())
                } else {
                    Err(CapacityError(value))
                }
            }
            #[cfg(feature = "alloc")]
            ManagedSlice::Owned(buf) => {
                buf.push(value);
                Ok(())
            }
        }
    }
}
