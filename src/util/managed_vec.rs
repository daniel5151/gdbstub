use managed::ManagedSlice;

/// Error value indicating insufficient capacity.
#[derive(Debug, Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
pub struct CapacityError<Element>(pub Element);

/// Wraps a ManagedSlice in a vec-like interface.
pub struct ManagedVec<'a, 'b, T> {
    buf: &'b mut ManagedSlice<'a, T>,
    len: usize,
}

impl<'a, 'b, T> ManagedVec<'a, 'b, T> {
    pub fn new_with_idx(buf: &'b mut ManagedSlice<'a, T>, len: usize) -> Self {
        ManagedVec { buf, len }
    }

    pub fn push(&mut self, value: T) -> Result<(), CapacityError<T>> {
        if self.len < self.buf.len() {
            self.buf[self.len] = value;
            self.len += 1;
            Ok(())
        } else {
            match &mut self.buf {
                ManagedSlice::Borrowed(_) => Err(CapacityError(value)),
                #[cfg(feature = "alloc")]
                ManagedSlice::Owned(buf) => {
                    buf.push(value);
                    Ok(())
                }
            }
        }
    }

    #[cfg(feature = "trace-pkt")]
    pub fn as_slice<'c: 'b>(&'c self) -> &'b [T] {
        &self.buf[..self.len]
    }
}
