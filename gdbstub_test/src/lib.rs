//! Automated regression testing framework for gdbstub.

#![warn(missing_docs)]

pub mod client;
pub mod process;

pub use client::GdbMiClient;
pub use process::EmulatorProcess;
pub use gdbstub_test_macros::gdbstub_test;

use std::path::PathBuf;

/// Helper to find the test ELF for a given example.
pub fn find_test_elf(example: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop(); // root
    path.push("examples");
    path.push(example);
    path.push("test_bin");
    path.push("test.elf");
    path
}
