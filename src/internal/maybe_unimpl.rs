use crate::{GdbStubError, OptResult};

#[derive(Debug, Clone)]
enum MaybeUnimplInner<E> {
    NoImpl,
    Error(E),
}

/// Wraps an error type with an additional "unimplemented" state. Can only be
/// constructed via `.into()` or the `?` operator. See
/// [`OptResult`](../type.OptResult.html) for more information.
#[derive(Debug, Clone)]
pub struct MaybeUnimpl<E>(MaybeUnimplInner<E>);

impl<E> MaybeUnimpl<E> {
    pub(crate) fn no_impl() -> Self {
        MaybeUnimpl(MaybeUnimplInner::NoImpl)
    }

    pub(crate) fn error(e: E) -> Self {
        MaybeUnimpl(MaybeUnimplInner::Error(e))
    }
}

/// utilities for working with OptResult in the `gdbstub` codebase.
pub(crate) trait OptResultExt<T, E> {
    /// Convert an `OptResult<T, E>` into a `Result<Option<T>,
    /// GdbStubError::TargetError<E>>`, where missing implementations return
    /// Ok(None).
    fn maybe_missing_impl<C>(self) -> Result<Option<T>, GdbStubError<E, C>>;
}

impl<T, E> OptResultExt<T, E> for OptResult<T, E> {
    fn maybe_missing_impl<C>(self) -> Result<Option<T>, GdbStubError<E, C>> {
        match self {
            Ok(t) => Ok(Some(t)),
            Err(MaybeUnimpl(MaybeUnimplInner::NoImpl)) => Ok(None),
            Err(MaybeUnimpl(MaybeUnimplInner::Error(e))) => Err(GdbStubError::TargetError(e)),
        }
    }
}

impl<T> From<T> for MaybeUnimpl<T> {
    fn from(e: T) -> Self {
        MaybeUnimpl(MaybeUnimplInner::Error(e))
    }
}
