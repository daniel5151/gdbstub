//! Library-defined implementations of the [`Connection`] trait for common types

#[cfg(feature = "std")]
mod tcpstream;

// TODO: impl Connection for all `Read + Write` (blocked on specialization)
