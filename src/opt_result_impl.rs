//! `OptResult` implementation details.

use crate::OptResult;

#[derive(Debug, Clone)]
enum MaybeNoImplInner<E> {
    NoImpl,
    Error(E),
}

/// Wraps an error type with an additional "unimplemented" state. Can only be
/// constructed via `.into()` or the `?` operator, and should be treated as a
/// opaque wrapper around the inner error.
#[derive(Debug, Clone)]
pub struct MaybeNoImpl<E>(MaybeNoImplInner<E>);

impl<E> MaybeNoImpl<E> {
    pub(crate) fn no_impl() -> Self {
        MaybeNoImpl(MaybeNoImplInner::NoImpl)
    }

    pub(crate) fn error(e: E) -> Self {
        MaybeNoImpl(MaybeNoImplInner::Error(e))
    }
}

/// utilities for working with OptResult in the `gdbstub` codebase.
pub(crate) trait OptResultExt<T, E> {
    /// Convert an `OptResult<T, E>` into a `Result<Option<T>,
    /// crate::Error::TargetError<E>>`, where missing implementations return
    /// Ok(None).
    fn maybe_missing_impl<C>(self) -> Result<Option<T>, crate::Error<E, C>>;
}

impl<T, E> OptResultExt<T, E> for OptResult<T, E> {
    fn maybe_missing_impl<C>(self) -> Result<Option<T>, crate::Error<E, C>> {
        match self {
            Ok(t) => Ok(Some(t)),
            Err(MaybeNoImpl(MaybeNoImplInner::NoImpl)) => Ok(None),
            Err(MaybeNoImpl(MaybeNoImplInner::Error(e))) => Err(crate::Error::TargetError(e)),
        }
    }
}

impl<T> From<T> for MaybeNoImpl<T> {
    fn from(e: T) -> Self {
        MaybeNoImpl(MaybeNoImplInner::Error(e))
    }
}
