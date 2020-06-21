//! Provides a vector that uses an external slice for storage.
//!
//! Forked from https://github.com/jonas-schievink/slicevec,
//! and tweaked to improve error types.

use core::borrow::{Borrow, BorrowMut};
use core::iter::FusedIterator;
use core::mem::{replace, swap};
use core::ops::{Deref, DerefMut};
use core::{cmp, slice};

/// Error value indicating insufficient capacity
#[derive(Debug, Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
pub struct CapacityError<Element>(pub Element);

/// A Vector using a slice for backing storage (passed in at creation time).
///
/// Changes to the vector are visible in the backing storage after the
/// `SliceVec` is dropped.
///
/// A `SliceVec` can be dereferenced to a truncated slice containing all
/// elements in the `SliceVec`. The returned slice is different from the backing
/// slice in that it only contains the first `n` values, where `n` is the
/// current length of the `SliceVec`. The backing slice may contain unused
/// "dummy" elements after the last element.
///
/// This is essentially a less ergonomic but more flexible version of the
/// `arrayvec` crate's `ArrayVec` type: You have to crate the backing storage
/// yourself, but `SliceVec` works with arrays of *any* length (unlike
/// `ArrayVec`, which works with a fixed set of lengths, since Rust
/// doesn't (yet) have integer generics).
#[derive(Debug)]
pub struct SliceVec<'a, T: 'a> {
    storage: &'a mut [T],
    len: usize,
}

impl<'a, T> SliceVec<'a, T> {
    /// Create a new `SliceVec`, using the given slice as backing storage for
    /// elements.
    ///
    /// The capacity of the vector equals the length of the slice, you have to
    /// make sure that the slice is large enough for all elements.
    pub fn new(storage: &'a mut [T]) -> Self {
        SliceVec { storage, len: 0 }
    }

    /// Returns the maximum number of elements that can be stored in this
    /// vector. This is equal to the length of the backing storage passed at
    /// creation of this `SliceVec`.
    pub fn capacity(&self) -> usize {
        self.storage.len()
    }

    /// Returns the number of elements stored in this `SliceVec`.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns `true` if the length of this vector is 0, `false` otherwise.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns `true` if the backing slice is completely filled.
    ///
    /// When this is the case, all operations that insert additional elements
    /// into the `SliceVec` will fail.
    pub fn is_full(&self) -> bool {
        self.len == self.storage.len()
    }

    /// Tries to append an element to the end of this vector.
    ///
    /// If the backing storage is already full, returns `Err(elem)`.
    pub fn push(&mut self, elem: T) -> Result<(), CapacityError<T>> {
        if self.len < self.capacity() {
            self.storage[self.len] = elem;
            self.len += 1;
            Ok(())
        } else {
            Err(CapacityError(elem))
        }
    }

    /// Removes and returns the last elements stored inside the vector,
    /// replacing it with `elem`.
    ///
    /// If the vector is empty, returns `None` and drops `elem`.
    pub fn pop_and_replace(&mut self, elem: T) -> Option<T> {
        // FIXME should this return a `Result<T, T>` instead?
        if self.len > 0 {
            self.len -= 1;
            let elem = replace(&mut self.storage[self.len], elem);
            Some(elem)
        } else {
            None
        }
    }

    /// Shortens the vector to `len` elements.
    ///
    /// Excess elements are not dropped. They are kept in the backing slice.
    pub fn truncate(&mut self, len: usize) {
        self.len = cmp::min(self.len, len);
    }

    /// Clears the vector, removing all elements.
    ///
    /// Equivalent to `.truncate(0)`.
    pub fn clear(&mut self) {
        self.truncate(0);
    }

    /// Extract a slice containing the entire vector.
    ///
    /// The returned slice will be shorter than the backing slice if the vector
    /// hasn't yet exceeded its capacity.
    pub fn as_slice(&self) -> &[T] {
        &self.storage[..self.len]
    }

    /// Extract a mutable slice containing the entire vector.
    ///
    /// The returned slice will be shorter than the backing slice if the vector
    /// hasn't yet exceeded its capacity.
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        &mut self.storage[..self.len]
    }
}

impl<'a, T: 'a + Default> SliceVec<'a, T> {
    /// Removes and returns the last element in this vector.
    ///
    /// Returns `None` if the vector is empty.
    ///
    /// This operation is restricted to element types that implement `Default`,
    /// since the element's spot in the backing storage is replaced by a
    /// default value.
    pub fn pop(&mut self) -> Option<T> {
        if self.len > 0 {
            self.len -= 1;
            let elem = core::mem::take(&mut self.storage[self.len]);
            Some(elem)
        } else {
            None
        }
    }

    /// Removes and returns the element at `index` and replaces it with the last
    /// element.
    ///
    /// The last element's place in the backing slice is replaced by `T`'s
    /// default value.
    ///
    /// Panics if `index` is out of bounds.
    pub fn swap_remove(&mut self, index: usize) -> T {
        let len = self.len();
        self.as_mut_slice().swap(index, len - 1);
        // the unwrap should never fail since we already touched the slice, causing a
        // bounds check
        self.pop().expect("swap_remove failed pop")
    }

    /// Removes and returns the element at `index` and shifts down all elements
    /// after it.
    ///
    /// Unlike `swap_remove`, this preserves the ordering of the vector, but is
    /// `O(n)` instead of `O(1)`.
    ///
    /// Panics if `index` is out of bounds.
    pub fn remove(&mut self, index: usize) -> T {
        // Just because I'm too lazy to reason about `unsafe` code, let's try something
        // else...
        assert!(index < self.len);

        // Swap all elements downwards until we arrive at `index`
        let mut replacement = T::default();
        for i in (index..self.len).rev() {
            swap(&mut self.storage[i], &mut replacement);
        }
        self.len -= 1;

        replacement
    }
}

impl<'a, T> Deref for SliceVec<'a, T> {
    type Target = [T];

    fn deref(&self) -> &[T] {
        self.as_slice()
    }
}

impl<'a, T> DerefMut for SliceVec<'a, T> {
    fn deref_mut(&mut self) -> &mut [T] {
        self.as_mut_slice()
    }
}

impl<'a, T: Default> IntoIterator for SliceVec<'a, T> {
    type Item = T;
    type IntoIter = IntoIter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter { v: self, next: 0 }
    }
}

impl<'a, T> IntoIterator for &'a SliceVec<'a, T> {
    type Item = &'a T;
    type IntoIter = slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.as_slice().iter()
    }
}

// Forward useful `[T]` impls so `SliceVec` is useful in generic contexts.
// TODO: There are a lot more we can forward. Is this the right way? `Vec<T>`
// also forwards dozens.

impl<'a, T> AsRef<[T]> for SliceVec<'a, T> {
    fn as_ref(&self) -> &[T] {
        self.as_slice()
    }
}

impl<'a, T> AsMut<[T]> for SliceVec<'a, T> {
    fn as_mut(&mut self) -> &mut [T] {
        self.as_mut_slice()
    }
}

impl<'a, T> Borrow<[T]> for SliceVec<'a, T> {
    fn borrow(&self) -> &[T] {
        self.as_slice()
    }
}

impl<'a, T> BorrowMut<[T]> for SliceVec<'a, T> {
    fn borrow_mut(&mut self) -> &mut [T] {
        self.as_mut_slice()
    }
}

/// An iterator that moves elements out of a `SliceVec`, replacing them with
/// their default value.
pub struct IntoIter<'a, T: 'a> {
    v: SliceVec<'a, T>,
    next: usize,
}

impl<'a, T: Default> FusedIterator for IntoIter<'a, T> {}
impl<'a, T: Default> Iterator for IntoIter<'a, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next < self.v.len() {
            let elem = core::mem::take(&mut self.v[self.next]);
            self.next += 1;
            Some(elem)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic() {
        const CAP: usize = 1;
        let mut storage = [0; CAP];

        {
            let mut s = SliceVec::new(&mut storage);
            assert!(s.is_empty());
            assert_eq!(s.len(), 0);
            assert_eq!(s.capacity(), CAP);

            assert_eq!(s.push(123), Ok(()));
            assert_eq!(s.len(), 1);
            assert!(!s.is_empty());
            assert_eq!(s.as_slice(), &[123]);
            assert_eq!(s.push(42), Err(CapacityError(42)));
            assert!(!s.is_empty());
            assert_eq!(s.as_slice(), &[123]);
            assert_eq!(s.pop(), Some(123));
            assert_eq!(s.len(), 0);
            assert!(s.is_empty());
            assert_eq!(s.as_slice(), &[]);
            assert_eq!(&*s, &[]);
        }
    }

    #[test]
    fn swap_remove() {
        let mut storage = [0; 5];
        let mut v = SliceVec::new(&mut storage);
        v.push(0).unwrap();
        v.push(1).unwrap();
        v.push(2).unwrap();
        v.push(3).unwrap();
        assert_eq!(v.as_slice(), &[0, 1, 2, 3]);
        assert_eq!(v.swap_remove(0), 0);
        assert_eq!(v.as_slice(), &[3, 1, 2]);
        assert_eq!(v.swap_remove(2), 2);
        assert_eq!(v.as_slice(), &[3, 1]);
        v.push(100).unwrap();
        v.push(101).unwrap();
        assert_eq!(v.as_slice(), &[3, 1, 100, 101]);
        assert_eq!(v.swap_remove(2), 100);
        assert_eq!(v.as_slice(), &[3, 1, 101]);
        v.push(102).unwrap();
        assert_eq!(v.as_slice(), &[3, 1, 101, 102]);
        assert_eq!(v.swap_remove(1), 1);
        assert_eq!(v.as_slice(), &[3, 102, 101]);
    }

    #[test]
    #[should_panic]
    fn swap_remove_empty() {
        let mut storage = [0; 5];
        let mut v = SliceVec::new(&mut storage);
        v.push(7).unwrap();
        v.clear();
        assert_eq!(v.as_slice(), &[]);
        assert!(v.is_empty());

        v.swap_remove(0);
    }

    #[test]
    #[should_panic]
    fn swap_remove_out_of_bounds() {
        let mut storage = [0; 5];
        let mut v = SliceVec::new(&mut storage);
        v.push(0).unwrap();
        v.push(1).unwrap();
        v.push(2).unwrap();
        v.push(3).unwrap();
        assert_eq!(v.as_slice(), &[0, 1, 2, 3]);

        v.swap_remove(4);
    }

    #[test]
    fn remove() {
        let mut storage = [0; 5];
        let mut v = SliceVec::new(&mut storage);
        v.push(0).unwrap();
        v.push(1).unwrap();
        v.push(2).unwrap();
        v.push(3).unwrap();
        assert_eq!(v.remove(0), 0);
        assert_eq!(v.as_slice(), &[1, 2, 3]);
        assert_eq!(v.remove(2), 3);
        assert_eq!(v.as_slice(), &[1, 2]);
        v.push(3).unwrap();
        v.push(4).unwrap();
        v.push(5).unwrap();
        assert_eq!(v.as_slice(), &[1, 2, 3, 4, 5]);
        assert_eq!(v.remove(1), 2);
        assert_eq!(v.as_slice(), &[1, 3, 4, 5]);
    }

    #[test]
    #[should_panic]
    fn remove_empty() {
        let mut storage = [0; 5];
        let mut v = SliceVec::new(&mut storage);
        v.push(7).unwrap();
        v.clear();
        assert_eq!(v.as_slice(), &[]);
        assert!(v.is_empty());

        v.remove(0);
    }

    #[test]
    #[should_panic]
    fn remove_out_of_bounds() {
        let mut storage = [0; 5];
        let mut v = SliceVec::new(&mut storage);
        v.push(0).unwrap();
        v.push(1).unwrap();
        v.push(2).unwrap();
        v.push(3).unwrap();
        assert_eq!(v.as_slice(), &[0, 1, 2, 3]);

        v.remove(4);
    }

    #[test]
    fn is_full() {
        let mut storage = [0; 3];
        let mut v = SliceVec::new(&mut storage);
        assert!(!v.is_full());
        v.push(0).unwrap();
        assert!(!v.is_full());
        v.push(1).unwrap();
        assert!(!v.is_full());
        v.push(2).unwrap();
        assert!(v.is_full());
        v.push(3).unwrap_err();
    }

    #[test]
    fn iter_with_for() {
        let mut storage = [0; 5];
        let mut v = SliceVec::new(&mut storage);
        v.push(0).unwrap();
        v.push(1).unwrap();
        v.push(2).unwrap();
        v.push(3).unwrap();

        let mut count = 0;
        for &elem in &v {
            assert_eq!(elem, count);
            count += 1;
        }
        let mut count = 0;
        for elem in v {
            assert_eq!(elem, count);
            count += 1;
        }
    }
}
