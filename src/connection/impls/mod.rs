//! Implementations of the [`Connection`] trait for various built-in types
// TODO: impl Connection for all `Read + Write` (blocked on specialization)

#[cfg(feature = "std")]
mod tcpstream;
