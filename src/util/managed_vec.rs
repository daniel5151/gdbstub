use managed::ManagedSlice;

/// Error value indicating insufficient capacity.
#[derive(Debug, Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
pub struct CapacityError<Element>(pub Element);

/// Wraps a ManagedSlice in a vec-like interface.
pub struct ManagedVec<'a, 'b, T: 'a> {
    buf: &'b mut ManagedSlice<'a, T>,
    len: usize,
}

impl<'a, 'b, T> ManagedVec<'a, 'b, T> {
    pub fn new(buf: &'b mut ManagedSlice<'a, T>) -> Self {
        ManagedVec { buf, len: 0 }
    }

    pub fn new_with_idx(buf: &'b mut ManagedSlice<'a, T>, len: usize) -> Self {
        ManagedVec { buf, len }
    }

    pub fn clear(&mut self) {
        // While it's very tempting to just call `Vec::clear` in the `Owned` case, doing
        // so would modify the length of the underlying `ManagedSlice`, which isn't
        // desirable.
        self.len = 0;
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

    pub fn as_slice<'c: 'b>(&'c self) -> &'b [T] {
        &self.buf[..self.len]
    }
}
