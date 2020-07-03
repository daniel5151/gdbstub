// Copyright (C) 2017 whitequark@whitequark.org
//
// Permission to use, copy, modify, and/or distribute this software for
// any purpose with or without fee is hereby granted.
//
// THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES
// WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
// MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR
// ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
// WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN
// AN ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT
// OF OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.

// Retrieved on Fri. Jul 3rd 2020.

// TODO: use crates.io version if/when smol-tcp/rust-managed#16 lands

use core::fmt;
use core::ops::{Deref, DerefMut};

#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::boxed::Box;
#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::vec::Vec;
#[cfg(feature = "std")]
use std::boxed::Box;
#[cfg(feature = "std")]
use std::vec::Vec;

/// A managed slice.
///
/// This enum can be used to represent exclusive access to slices of objects.
/// In Rust, exclusive access to an object is obtained by either owning the
/// object, or owning a mutable pointer to the object; hence, "managed".
///
/// The purpose of this enum is providing good ergonomics with `std` present
/// while making it possible to avoid having a heap at all (which of course
/// means that `std` is not present). To achieve this, the variants other than
/// `Borrowed` are only available when the corresponding feature is opted in.
///
/// A function that requires a managed object should be generic over an
/// `Into<ManagedSlice<'a, T>>` argument; then, it will be possible to pass
/// either a `Vec<T>`, or a `&'a mut [T]` without any conversion at the call
/// site.
///
/// See also [Managed](enum.Managed.html).
pub enum ManagedSlice<'a, T: 'a> {
    /// Borrowed variant.
    Borrowed(&'a mut [T]),
    /// Owned variant, only available with the `std` or `alloc` feature enabled.
    #[cfg(any(feature = "std", feature = "alloc"))]
    Owned(Vec<T>),
}

impl<'a, T: 'a> fmt::Debug for ManagedSlice<'a, T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ManagedSlice::Borrowed(ref x) => write!(f, "Borrowed({:?})", x),
            #[cfg(any(feature = "std", feature = "alloc"))]
            ManagedSlice::Owned(ref x) => write!(f, "Owned({:?})", x),
        }
    }
}

impl<'a, T: 'a> From<&'a mut [T]> for ManagedSlice<'a, T> {
    fn from(value: &'a mut [T]) -> Self {
        ManagedSlice::Borrowed(value)
    }
}

#[cfg(any(feature = "std", feature = "alloc"))]
macro_rules! from_unboxed_slice {
    ($n:expr) => (
        impl<'a, T> From<[T; $n]> for ManagedSlice<'a, T> {
            #[inline]
            fn from(value: [T; $n]) -> Self {
                ManagedSlice::Owned((Box::new(value) as Box<[T]>).into_vec())
            }
        }
    );
    ($n:expr, $( $r:expr ),*) => (
        from_unboxed_slice!($n);
        from_unboxed_slice!($( $r ),*);
    )
}

#[cfg(any(feature = "std", feature = "alloc"))]
from_unboxed_slice!(
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
    26, 27, 28, 29, 30, 31
);

#[cfg(any(feature = "std", feature = "alloc"))]
impl<'a, T: 'a> From<Vec<T>> for ManagedSlice<'a, T> {
    fn from(value: Vec<T>) -> Self {
        ManagedSlice::Owned(value)
    }
}

impl<'a, T: 'a> Deref for ManagedSlice<'a, T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        match self {
            ManagedSlice::Borrowed(ref value) => value,
            #[cfg(any(feature = "std", feature = "alloc"))]
            ManagedSlice::Owned(ref value) => value,
        }
    }
}

impl<'a, T: 'a> DerefMut for ManagedSlice<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            ManagedSlice::Borrowed(ref mut value) => value,
            #[cfg(any(feature = "std", feature = "alloc"))]
            ManagedSlice::Owned(ref mut value) => value,
        }
    }
}
