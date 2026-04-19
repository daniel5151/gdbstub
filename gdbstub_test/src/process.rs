//! Emulator process management for gdbstub regression testing.

use std::process::{Child, Command, Stdio};
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

/// A running emulator process.
pub struct EmulatorProcess {
    child: Child,
    uds_path: PathBuf,
}

impl EmulatorProcess {
    /// Spawns a new emulator process for the given example.
    pub fn spawn(example: &str) -> std::io::Result<Self> {
        let uds_path = PathBuf::from("/tmp/armv4t_gdb");
        
        // Robust cleanup: try to remove and wait
        for _ in 0..10 {
            if !uds_path.exists() { break; }
            let _ = std::fs::remove_file(&uds_path);
            std::thread::sleep(std::time::Duration::from_millis(100));
        }

        let mut manifest_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        manifest_path.pop(); // root
        manifest_path.push("Cargo.toml");

        let mut command = if std::env::var("CARGO_LLVM_COV").is_ok() {
            let mut c = Command::new("cargo");
            c.arg("llvm-cov")
                .arg("run")
                .arg("-q")
                .arg("--manifest-path")
                .arg(manifest_path)
                .arg("--example")
                .arg(example)
                .arg("--")
                .arg("--uds");
            c
        } else {
            let mut c = Command::new("cargo");
            c.arg("run")
                .arg("-q")
                .arg("--manifest-path")
                .arg(manifest_path)
                .arg("--example")
                .arg(example)
                .arg("--")
                .arg("--uds");
            c
        };

        command.stdout(Stdio::inherit())
            .stderr(Stdio::piped());

        // Propagate environment
        command.envs(std::env::vars());

        let mut child = command.spawn()?;

        let mut stderr_reader = BufReader::new(child.stderr.take().unwrap());
        let mut line = String::new();
        
        // Wait for "Waiting for a GDB connection"
        let mut found = false;
        let mut captured_stderr = String::new();
        loop {
            line.clear();
            if stderr_reader.read_line(&mut line)? == 0 {
                break;
            }
            captured_stderr.push_str(&line);
            if line.contains("Waiting for a GDB connection") {
                found = true;
                break;
            }
        }

        if !found {
            let _ = child.kill();
            return Err(std::io::Error::other(format!(
                "Emulator failed to start or didn't output expected ready message. Stderr was:\n{}",
                captured_stderr
            )));
        }

        // Spawn a thread to forward the rest of stderr
        std::thread::spawn(move || {
            let mut line = String::new();
            while let Ok(n) = stderr_reader.read_line(&mut line) {
                if n == 0 { break; }
                eprint!("EMU: {}", line);
                line.clear();
            }
        });

        // Wait for socket to appear
        let mut count = 0;
        while count < 50 {
            if uds_path.exists() { break; }
            std::thread::sleep(std::time::Duration::from_millis(100));
            count += 1;
        }
        
        // Give it one more tiny bit to be absolutely sure bind() is done
        std::thread::sleep(std::time::Duration::from_millis(200));

        Ok(Self {
            child,
            uds_path,
        })
    }

    /// Returns the path to the Unix Domain Socket.
    pub fn uds_path(&self) -> &PathBuf {
        &self.uds_path
    }
}

impl Drop for EmulatorProcess {
    fn drop(&mut self) {
        #[cfg(unix)]
        {
            // Use SIGTERM to allow graceful exit and coverage data writing
            unsafe {
                libc::kill(self.child.id() as i32, libc::SIGTERM);
            }

            // Give it a long moment to exit and flush coverage data
            let _ = self.child.wait();
            std::thread::sleep(std::time::Duration::from_secs(2));
        }
        let _ = self.child.kill();
    }
}
