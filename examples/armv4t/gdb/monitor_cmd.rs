use gdbstub::target_ext;
use gdbstub::target_ext::monitor_cmd::{outputln, ConsoleOutput};

use crate::gdb::Emu;

impl target_ext::monitor_cmd::MonitorCmd for Emu {
    fn handle_monitor_cmd(
        &mut self,
        cmd: &[u8],
        mut out: ConsoleOutput<'_>,
    ) -> Result<(), Self::Error> {
        let cmd = match core::str::from_utf8(cmd) {
            Ok(cmd) => cmd,
            Err(_) => {
                outputln!(out, "command must be valid UTF-8");
                return Ok(());
            }
        };

        match cmd {
            "" => outputln!(out, "Sorry, didn't catch that. Try `monitor ping`!"),
            "ping" => outputln!(out, "pong!"),
            _ => outputln!(out, "I don't know how to handle '{}'", cmd),
        };

        Ok(())
    }
}
