use crate::gdb::Emu;
use gdbstub::target;
use gdbstub::target::ext::monitor_cmd::outputln;
use gdbstub::target::ext::monitor_cmd::ConsoleOutput;

impl target::ext::monitor_cmd::MonitorCmd for Emu {
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

        let mut args = cmd.split(' ');

        match args.next() {
            None => outputln!(out, "Sorry, didn't catch that. Try `monitor ping`!"),
            Some("ping") => outputln!(out, "pong!"),
            Some("fake-exec") => {
                let Some(path) = args.next() else {
                    outputln!(
                        out,
                        "expected fake arg (likely /test.elf, to match `exec_file`)"
                    );
                    return Ok(());
                };

                self.fake_exec = Some(path.into());

                outputln!(
                    out,
                    "ok, will report `exec` stop reason (with {path} as the path) when resumed!"
                )
            }
            _ => outputln!(out, "I don't know how to handle '{}'", cmd),
        };

        Ok(())
    }
}
