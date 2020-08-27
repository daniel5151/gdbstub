//! Handle custom commands sent using GDB's `monitor` command.

use crate::target::Target;

pub use crate::protocol::ConsoleOutput;
pub use crate::{output, outputln};

/// Target Extension - Handle custom GDB `monitor` commands.
pub trait MonitorCmd: Target {
    /// Handle custom commands sent using the `monitor` command.
    ///
    /// The GDB remote serial protocol includes a built-in mechanism to send
    /// arbitrary commands to the remote stub: the `monitor` command. For
    /// example, running `monitor dbg` from the GDB client will invoke
    /// `handle_monitor_cmd` with `cmd = b"dbg"`.
    ///
    /// Commands are _not_ guaranteed to be valid UTF-8, hence the use of
    /// `&[u8]` as opposed to `&str`.
    ///
    /// Intermediate console output can be written back to the GDB client using
    /// the provided `ConsoleOutput` object + the
    /// [`gdbstub::output!`](macro.output.html) macro.
    ///
    /// _Note:_ The maximum length of incoming commands is limited by the size
    /// of the packet buffer provided to the [`GdbStub`](struct.GdbStub.html).
    /// Specifically, commands can only be up to `(buf.len() - 10) / 2` bytes.
    fn handle_monitor_cmd(&mut self, cmd: &[u8], out: ConsoleOutput<'_>)
        -> Result<(), Self::Error>;
}
