use core::fmt;

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

/// Helper struct to send console output to GDB.
///
/// The recommended way to interact with `ConsoleOutput` is through the provided
/// [`output!`] and [`outputln!`] macros.
///
/// On resource constrained systems which might want to avoid using Rust's
/// [fairly "heavy" formatting machinery](https://jamesmunns.com/blog/fmt-unreasonably-expensive/),
/// the `write_raw()` method can be used to write raw data directly to the GDB
/// console.
///
/// When the `alloc` feature is disabled, all output buffering is disabled, and
/// each call to `output!` will automatically flush data over the Connection.
///
/// [`output!`]: crate::output
/// [`outputln!`]: crate::outputln
// TODO: support user-provided output buffers for no-`alloc` environments.
pub struct ConsoleOutput<'a> {
    #[cfg(feature = "alloc")]
    buf: Vec<u8>,
    callback: &'a mut dyn FnMut(&[u8]),
}

impl<'a> fmt::Write for ConsoleOutput<'a> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_raw(s.as_bytes());
        Ok(())
    }
}

impl<'a> ConsoleOutput<'a> {
    pub(crate) fn new(callback: &'a mut dyn FnMut(&[u8])) -> ConsoleOutput<'a> {
        ConsoleOutput {
            #[cfg(feature = "alloc")]
            buf: Vec::new(),
            callback,
        }
    }

    /// Write raw (non UTF-8) data to the GDB console.
    pub fn write_raw(&mut self, bytes: &[u8]) {
        cfg_if::cfg_if! {
            if #[cfg(feature = "alloc")] {
                self.buf.extend_from_slice(bytes);
            } else {
                (self.callback)(bytes);
            }
        }
    }

    /// Flush the internal output buffer.
    ///
    /// Only available when `alloc` is enabled.
    #[cfg(feature = "alloc")]
    pub fn flush(&mut self) {
        if !self.buf.is_empty() {
            (self.callback)(&self.buf);
            self.buf.clear()
        }
    }
}

impl Drop for ConsoleOutput<'_> {
    fn drop(&mut self) {
        #[cfg(feature = "alloc")]
        self.flush()
    }
}

/// Send formatted data to the GDB client console.
///
/// The first argument must be a [`ConsoleOutput`].
#[macro_export]
macro_rules! output {
    ($console_output:expr, $($args:tt)*) => {{
        use core::fmt::Write;
        let _ = write!($console_output, $($args)*);
    }};
}

/// Send formatted data to the GDB client console, with a newline appended.
///
/// The first argument must be a [`ConsoleOutput`].
#[macro_export]
macro_rules! outputln {
    ($console_output:expr) => {{
        use core::fmt::Write;
        let _ = writeln!($console_output);
    }};
    ($console_output:expr,) => {
        outputln!($console_output)
    };
    ($console_output:expr, $($args:tt)*) => {{
        use core::fmt::Write;
        let _ = writeln!($console_output, $($args)*);
    }};
}
