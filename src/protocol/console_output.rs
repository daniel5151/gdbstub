use core::fmt;

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

/// Helper struct to send console output to GDB.
///
/// The recommended way to interact with `ConsoleOutput` is through the provided
/// [`output!`](macro.outputln.html) / [`outputln!`](macro.outputln.html)
/// macros.
///
/// When the `alloc` feature is disabled, all output buffering is disabled, and
/// each `write` call will automatically flush data over the Connection.
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
/// The first argument must be a [`ConsoleWriter`](struct.ConsoleWriter.html).
#[macro_export]
macro_rules! output {
    ($console_output:expr, $($args:tt)*) => {{
        use std::fmt::Write;
        writeln!($console_output, $($args)*).unwrap();
    }};
}

/// Send formatted data to the GDB client console, with a newline appended.
///
/// The first argument must be a [`ConsoleWriter`](struct.ConsoleWriter.html).
#[macro_export]
macro_rules! outputln {
    ($console_output:expr) => {{
        use std::fmt::Write;
        let _ = writeln!($console_output);
    }};
    ($console_output:expr,) => {
        outputln!($console_output)
    };
    ($console_output:expr, $($args:tt)*) => {{
        use std::fmt::Write;
        writeln!($console_output, $($args)*).unwrap();
    }};
}
