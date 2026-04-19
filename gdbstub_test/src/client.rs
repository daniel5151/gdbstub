//! GDB/MI client for programmatic control of GDB.

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::env;
use std::time::Duration;
use std::thread;

use anyhow::{anyhow, Context, Result};
use crossbeam_channel::Receiver;

/// A GDB client using the Machine Interface (MI).
pub struct GdbMiClient {
    child: Child,
    stdin: ChildStdin,
    rx: Receiver<String>,
}

/// Represents a GDB/MI stop reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StopReason {
    /// Signal received.
    Signal(String),
    /// Breakpoint hit.
    BreakpointHit(u32),
    /// Step finished.
    EndSteppingRange,
    /// Other reasons.
    Other(String),
}

impl StopReason {
    /// Returns true if the target is still alive after this stop.
    pub fn is_alive(&self) -> bool {
        !matches!(self, StopReason::Signal(s) if s == "SIGKILL" || s == "SIGTERM")
    }
}

/// Information about a thread.
#[derive(Debug, Clone)]
pub struct ThreadInfo {
    /// Thread ID.
    pub id: u32,
    /// Extra information (e.g. core name).
    pub extra_info: Option<String>,
}

impl GdbMiClient {
    /// Spawns a new GDB process.
    pub fn spawn(gdb_path: Option<&str>, target_elf: &str) -> Result<Self> {
        let gdb_bin = gdb_path
            .map(|s| s.to_string())
            .or_else(|| env::var("GDB_PATH").ok())
            .unwrap_or_else(|| "gdb-multiarch".to_string());

        let mut child = Command::new(gdb_bin)
            .arg("--interpreter=mi2")
            .arg("-q")
            .arg(target_elf)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .context("failed to spawn gdb process")?;

        let stdin = child.stdin.take().ok_or_else(|| anyhow!("failed to take stdin"))?;
        let stdout = child.stdout.take().ok_or_else(|| anyhow!("failed to take stdout"))?;

        let (tx, rx) = crossbeam_channel::unbounded();

        // Background thread to drain stdout and prevent pipe buffer deadlocks
        thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                if let Ok(line) = line {
                    let _ = tx.send(line);
                } else {
                    break;
                }
            }
        });

        let mut client = Self {
            child,
            stdin,
            rx,
        };

        // Standard setup for stub testing
        client.exec_cmd("-gdb-set breakpoint auto-hw off")?;
        client.exec_cmd("-gdb-set confirm off")?;
        client.exec_cmd("-gdb-set width 0")?;
        client.exec_cmd("-gdb-set height 0")?;

        Ok(client)
    }

    /// Writes a raw command to GDB's stdin.
    pub fn write_raw(&mut self, cmd: &str) -> Result<()> {
        log::trace!("GDB << {}", cmd);
        self.stdin.write_all(cmd.as_bytes())?;
        self.stdin.write_all(b"\n")?;
        self.stdin.flush().context("failed to flush gdb stdin")
    }

    /// Reads a line from the background receiver with a timeout.
    pub fn read_line_timeout(&mut self) -> Result<String> {
        self.rx.recv_timeout(Duration::from_secs(10))
            .context("GDB response timeout (10s exceeded)")
    }

    /// Executes a command and waits for a result record (^done, ^error, etc).
    /// Accumulates stream records (~, &, @) into the returned string.
    pub fn exec_cmd(&mut self, cmd: &str) -> Result<String> {
        self.write_raw(cmd)?;
        let mut output = String::new();
        loop {
            let res = self.read_line_timeout()?;
            let trimmed = res.trim();
            if trimmed == "(gdb)" || trimmed.is_empty() {
                continue;
            }

            if let Some(rest) = res.strip_prefix("^done") {
                output.push_str(rest.trim_start_matches(','));
                return Ok(output);
            }
            if let Some(rest) = res.strip_prefix("^running") {
                output.push_str(rest.trim_start_matches(','));
                return Ok(output);
            }
            if let Some(rest) = res.strip_prefix("^connected") {
                output.push_str(rest.trim_start_matches(','));
                return Ok(output);
            }
            if let Some(rest) = res.strip_prefix("^error") {
                return Err(anyhow!("GDB command '{}' failed: {} (log: {})", cmd, rest.trim_start_matches(','), output));
            }
            
            // Accumulate stream records
            if let Some(rest) = res.strip_prefix('~') {
                output.push_str(&self.unescape_mi_string(rest.trim()));
            } else if let Some(rest) = res.strip_prefix('&') {
                output.push_str(&self.unescape_mi_string(rest.trim()));
            }
        }
    }

    fn unescape_mi_string(&self, s: &str) -> String {
        let s = s.trim_matches('"');
        let mut result = String::new();
        let mut chars = s.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\\' {
                match chars.peek() {
                    Some(&'n') => { result.push('\n'); chars.next(); }
                    Some(&'t') => { result.push('\t'); chars.next(); }
                    Some(&'\"') => { result.push('"'); chars.next(); }
                    Some(&'\\') => { result.push('\\'); chars.next(); }
                    Some(&c) if c.is_digit(8) => {
                        let mut octal = String::new();
                        for _ in 0..3 {
                            if let Some(&c) = chars.peek() {
                                if c.is_digit(8) {
                                    octal.push(c);
                                    chars.next();
                                } else {
                                    break;
                                }
                            }
                        }
                        if let Ok(val) = u8::from_str_radix(&octal, 8) {
                            result.push(val as char);
                        }
                    }
                    _ => { result.push('\\'); }
                }
            } else {
                result.push(c);
            }
        }
        result
    }

    /// Connects to a remote target via UDS.
    pub fn connect_uds(&mut self, path: &str) -> Result<()> {
        let cmd = format!("target remote {}", path);
        let mut count = 0;
        loop {
            match self.exec_cmd(&cmd) {
                Ok(_) => return Ok(()),
                Err(e) if count < 10 => {
                    std::thread::sleep(Duration::from_millis(200));
                    count += 1;
                    continue;
                }
                Err(e) => return Err(e).context("failed to connect to target after retries"),
            }
        }
    }

    /// Connects to a remote target via UDS using extended-remote.
    pub fn connect_extended_uds(&mut self, path: &str) -> Result<()> {
        let cmd = format!("target extended-remote {}", path);
        let mut count = 0;
        loop {
            match self.exec_cmd(&cmd) {
                Ok(_) => return Ok(()),
                Err(e) if count < 10 => {
                    std::thread::sleep(Duration::from_millis(200));
                    count += 1;
                    continue;
                }
                Err(e) => return Err(e).context("failed to connect to target in extended mode after retries"),
            }
        }
    }

    /// Returns the current value of the program counter ($pc).
    pub fn get_pc(&mut self) -> Result<u32> {
        let res = self.exec_cmd("-data-evaluate-expression $pc")?;
        self.parse_value(&res)
    }

    fn parse_value(&self, res: &str) -> Result<u32> {
        // GDB MI might return value in different formats:
        // 1. value="0x1234"
        // 2. value="4660"
        // 3. value="0x1234 <main>"
        if let Some(pos) = res.find("value=\"") {
            let start = pos + 7;
            if let Some(end) = res[start..].find("\"") {
                let val_str = &res[start..start + end];
                let val_str = val_str.split_whitespace().next().unwrap_or(val_str);
                
                if let Some(hex_str) = val_str.strip_prefix("0x") {
                    return u32::from_str_radix(hex_str, 16).map_err(|e| anyhow!(e));
                }
                
                // Try parsing as hex if it contains a-f
                if val_str.chars().any(|c| c.is_ascii_alphabetic()) {
                     return u32::from_str_radix(val_str, 16).map_err(|e| anyhow!(e));
                }

                // Try parsing as decimal (could be signed)
                if let Ok(val) = val_str.parse::<i32>() {
                    return Ok(val as u32);
                }
                
                if let Ok(val) = val_str.parse::<u32>() {
                    return Ok(val);
                }
                
                // Fallback to hex
                return u32::from_str_radix(val_str, 16).map_err(|e| anyhow!("Failed to parse '{}' as u32: {}", val_str, e));
            }
        }
        Err(anyhow!("Failed to find value in: {}", res))
    }

    /// Single-steps the target.
    pub fn step(&mut self) -> Result<StopReason> {
        self.exec_cmd("-exec-step-instruction")?;
        self.wait_for_stop()
    }

    /// Single-steps a specific thread with a signal.
    pub fn step_with_signal(&mut self, tid: u32, sig: u8) -> Result<StopReason> {
        self.exec_cmd(&format!("-exec-step-instruction --thread {} --signal {}", tid, sig))?;
        self.wait_for_stop()
    }

    /// Continues execution until a stop event.
    pub fn continue_exec(&mut self) -> Result<StopReason> {
        self.exec_cmd("-exec-continue")?;
        self.wait_for_stop()
    }

    /// Waits for a stop notification.
    pub fn wait_for_stop(&mut self) -> Result<StopReason> {
        loop {
            let line = self.read_line_timeout()?;
            if let Some(pos) = line.find("*stopped") {
                let stopped_payload = &line[pos..];
                if stopped_payload.contains("reason=\"end-stepping-range\"") {
                    return Ok(StopReason::EndSteppingRange);
                }
                if stopped_payload.contains("reason=\"breakpoint-hit\"") {
                    if let Some(bkpt_pos) = stopped_payload.find("bkptno=\"") {
                        let start = bkpt_pos + 8;
                        if let Some(end) = stopped_payload[start..].find("\"") {
                            let no = stopped_payload[start..start+end].parse().unwrap_or(0);
                            return Ok(StopReason::BreakpointHit(no));
                        }
                    }
                    return Ok(StopReason::BreakpointHit(0));
                }
                if stopped_payload.contains("reason=\"exited-normally\"") {
                    return Ok(StopReason::Other("exited-normally".to_string()));
                }
                if stopped_payload.contains("reason=\"signal-received\"") {
                    return Ok(StopReason::Signal("unknown".to_string()));
                }
                return Ok(StopReason::Other(stopped_payload.to_string()));
            }
        }
    }

    /// Reads memory from the target.
    pub fn read_mem(&mut self, addr: u32, len: usize) -> Result<Vec<u8>> {
        let res = self.exec_cmd(&format!("-data-read-memory-bytes {:#x} {}", addr, len))?;
        if let Some(pos) = res.find("contents=\"") {
            let start = pos + 10;
            if let Some(end) = res[start..].find("\"") {
                let hex_str = &res[start..start + end];
                let mut data = Vec::with_capacity(hex_str.len() / 2);
                for i in (0..hex_str.len()).step_by(2) {
                    let b = u8::from_str_radix(&hex_str[i..i + 2], 16)?;
                    data.push(b);
                }
                return Ok(data);
            }
        }
        Err(anyhow!("Failed to parse memory from: {}", res))
    }

    /// Writes memory to the target.
    pub fn write_mem(&mut self, addr: u32, data: &[u8]) -> Result<()> {
        let mut hex_str = String::with_capacity(data.len() * 2);
        for b in data {
            hex_str.push_str(&format!("{:02x}", b));
        }
        self.exec_cmd(&format!("-data-write-memory-bytes {:#x} {}", addr, hex_str))?;
        Ok(())
    }

    /// Sets a software breakpoint.
    pub fn set_breakpoint(&mut self, addr: u32) -> Result<()> {
        self.exec_cmd(&format!("-break-insert *{:#x}", addr))?;
        Ok(())
    }

    /// Sets a hardware watchpoint.
    pub fn set_watchpoint(&mut self, addr: u32, len: usize, kind: &str) -> Result<()> {
        let flag = match kind {
            "rwatch" => "-r",
            "awatch" => "-a",
            _ => "",
        };
        // Note: 'len' is currently ignored as GDB watchpoint syntax doesn't always require it for address-based watchpoints
        self.exec_cmd(&format!("-break-watch {} *{:#x}", flag, addr))?;
        Ok(())
    }

    /// Sends a monitor command.
    pub fn monitor(&mut self, cmd: &str) -> Result<String> {
        self.exec_cmd(&format!("-interpreter-exec console \"monitor {}\"", cmd))
    }

    /// Reads a register by name.
    pub fn read_reg(&mut self, name: &str) -> Result<u32> {
        let res = self.exec_cmd(&format!("-data-evaluate-expression ${}", name))?;
        self.parse_value(&res)
    }

    /// Writes a register by name.
    pub fn write_reg(&mut self, name: &str, val: u32) -> Result<()> {
        self.exec_cmd(&format!("-interpreter-exec console \"set ${} = {:#x}\"", name, val))?;
        Ok(())
    }

    /// Switches the current thread.
    pub fn select_thread(&mut self, id: u32) -> Result<()> {
        self.exec_cmd(&format!("-thread-select {}", id))?;
        Ok(())
    }

    /// Gets information about a thread.
    pub fn get_thread_info(&mut self, id: u32) -> Result<ThreadInfo> {
        let res = self.exec_cmd(&format!("-thread-info {}", id))?;
        let extra_info = if let Some(pos) = res.find("details=\"") {
            let start = pos + 9;
            if let Some(end) = res[start..].find("\"") {
                Some(res[start..start+end].to_string())
            } else {
                None
            }
        } else {
            None
        };
        Ok(ThreadInfo {
            id,
            extra_info,
        })
    }

    /// Single-steps a specific thread.
    pub fn step_thread(&mut self, tid: u32) -> Result<StopReason> {
        self.exec_cmd(&format!("-exec-step-instruction --thread {}", tid))?;
        self.wait_for_stop()
    }

    /// Continues a specific thread.
    pub fn continue_thread(&mut self, tid: u32) -> Result<StopReason> {
        self.exec_cmd(&format!("-exec-continue --thread {}", tid))?;
        self.wait_for_stop()
    }

    /// Removes a software breakpoint.
    pub fn remove_breakpoint(&mut self, addr: u32) -> Result<()> {
        self.exec_cmd(&format!("-break-delete *{:#x}", addr))?;
        Ok(())
    }

    /// Sets a hardware watchpoint.
    pub fn set_watchpoint_typed(&mut self, addr: u32, kind: &str) -> Result<()> {
        let flag = match kind {
            "rwatch" => "-r",
            "awatch" => "-a",
            _ => "",
        };
        self.exec_cmd(&format!("-break-watch {} *{:#x}", flag, addr))?;
        Ok(())
    }

    /// Removes a watchpoint.
    pub fn remove_watchpoint(&mut self, addr: u32) -> Result<()> {
        self.exec_cmd(&format!("-interpreter-exec console \"delete watchpoint *{:#x}\"", addr))?;
        Ok(())
    }

    /// Deletes a file on the remote target.
    pub fn delete_remote_file(&mut self, path: &str) -> Result<()> {
        self.exec_cmd(&format!("-interpreter-exec console \"remote delete {}\"", path))?;
        Ok(())
    }

    /// Returns information about a specific register (LLDB extension).
    pub fn get_register_info(&mut self, id: usize) -> Result<String> {
        self.exec_cmd(&format!("-data-list-register-info {}", id))
    }

    /// Returns a list of active thread IDs.
    pub fn list_threads(&mut self) -> Result<Vec<u32>> {
        let res = self.exec_cmd("-thread-list-ids")?;
        let mut tids = Vec::new();
        let mut current = &res[..];
        while let Some(pos) = current.find("thread-id=\"") {
            let start = pos + 11;
            if let Some(end) = current[start..].find("\"") {
                if let Ok(tid) = current[start..start+end].parse() {
                    tids.push(tid);
                }
                current = &current[start+end+1..];
            } else {
                break;
            }
        }
        Ok(tids)
    }

    /// Interrupts the target.
    pub fn interrupt(&mut self) -> Result<()> {
        self.exec_cmd("-exec-interrupt")?;
        let _ = self.wait_for_stop();
        Ok(())
    }

    /// Detaches from the target.
    pub fn detach(&mut self) -> Result<()> {
        self.exec_cmd("-target-detach")?;
        Ok(())
    }

    /// Kills the target.
    pub fn kill(&mut self) -> Result<()> {
        self.exec_cmd("-interpreter-exec console \"kill\"")?;
        Ok(())
    }

    /// Sets a hardware breakpoint.
    pub fn set_hw_breakpoint(&mut self, addr: u32) -> Result<()> {
        self.exec_cmd(&format!("-break-insert -h *{:#x}", addr))?;
        Ok(())
    }

    /// Sets a catchpoint for syscalls.
    pub fn set_catch_syscall(&mut self, syscalls: Option<&[&str]>) -> Result<()> {
        let args = syscalls.map(|s| s.join(" ")).unwrap_or_default();
        self.exec_cmd(&format!("-interpreter-exec console \"catch syscall {}\"", args))?;
        Ok(())
    }

    /// Starts a trace experiment.
    pub fn trace_start(&mut self) -> Result<()> {
        self.exec_cmd("-interpreter-exec console \"tstart\"")?;
        Ok(())
    }

    /// Stops a trace experiment.
    pub fn trace_stop(&mut self) -> Result<()> {
        self.exec_cmd("-interpreter-exec console \"tstop\"")?;
        Ok(())
    }

    /// Gets the entry point of the current executable.
    pub fn get_entry_point(&mut self) -> Result<u32> {
        let res = self.exec_cmd("-interpreter-exec console \"info files\"")?;
        if let Some(pos) = res.find("Entry point: ") {
            let start = pos + 13;
            let val_str_raw = &res[start..];
            let end = val_str_raw.find(|c: char| c.is_whitespace() || c == '\n').unwrap_or(val_str_raw.len());
            let val_str = val_str_raw[..end].trim().strip_prefix("0x").unwrap_or(val_str_raw[..end].trim());
            return u32::from_str_radix(val_str, 16).map_err(|e| anyhow!("parse error for '{}': {}", val_str, e));
        }
        Err(anyhow!("Failed to find entry point in: {}", res))
    }
}

impl Drop for GdbMiClient {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}
