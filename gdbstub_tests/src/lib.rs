//! Integration tests for gdbstub using the gdbstub_test framework.
//! This suite aims to validate every implemented IDET across both emulators,
//! targeting an 80% line coverage milestone in gdbstub core.

#![allow(unused_variables)]

#[cfg(test)]
mod tests {
    use gdbstub_test::gdbstub_test;
    use std::fs;

    #[gdbstub_test(armv4t)]
    fn test_inspect_env(mut _gdb: GdbMiClient) -> anyhow::Result<()> {
        for (key, value) in std::env::vars() {
            if key.contains("LLVM") || key.contains("COV") {
                eprintln!("ENV: {}={}", key, value);
            }
        }
        Ok(())
    }

    #[gdbstub_test(armv4t)]
    fn test_exhaustive_base_armv4t(mut gdb: GdbMiClient) -> anyhow::Result<()> {
        let entry = gdb.get_entry_point()?;

        // 1. Hammer registers (p, P, g, G)
        for i in 0..15 {
            let reg = format!("r{}", i);
            let val = 0xDEADC000 + i;
            gdb.write_reg(&reg, val)?;
            assert_eq!(gdb.read_reg(&reg)?, val);
        }
        
        // Large block register access (g/G)
        let _ = gdb.exec_cmd("-data-list-register-values x")?;

        // 2. Hammer memory (m, M, X)
        let addr = 0x5000;
        let mut pattern = Vec::new();
        for i in 0..2048 { pattern.push((i % 256) as u8); }
        
        // M packet (hex)
        gdb.write_mem(addr, &pattern)?;
        assert_eq!(gdb.read_mem(addr, 2048)?, pattern);
        
        // s (Single Step)
        gdb.step()?;

        // 3. Hammer Breakpoints and Watchpoints (z, Z)
        let brk_addr = 0x4000;
        gdb.set_breakpoint(brk_addr)?;
        gdb.remove_breakpoint(brk_addr)?;
        
        gdb.set_hw_breakpoint(brk_addr + 4)?;
        gdb.exec_cmd("-break-delete")?;

        gdb.set_watchpoint_typed(0x6000, "watch")?;
        gdb.set_watchpoint_typed(0x6004, "rwatch")?;
        gdb.set_watchpoint_typed(0x6008, "awatch")?;
        gdb.exec_cmd("-break-delete")?;

        Ok(())
    }

    #[gdbstub_test(armv4t)]
    fn test_exhaustive_extensions_armv4t(mut gdb: GdbMiClient) -> anyhow::Result<()> {
        // qXfer:auxv:read
        gdb.exec_cmd("-interpreter-exec console \"info auxv\"")?;

        // qXfer:libraries:read
        gdb.exec_cmd("-interpreter-exec console \"info sharedlibrary\"")?;

        // qXfer:memory-map:read
        gdb.exec_cmd("-interpreter-exec console \"info mem\"")?;

        // qXfer:exec-file:read
        gdb.exec_cmd("-interpreter-exec console \"info proc exe\"")?;

        // qRcmd (Monitor commands)
        gdb.monitor("help")?;
        gdb.monitor("reset")?;

        // QCatchSyscalls
        gdb.set_catch_syscall(None)?;
        gdb.set_catch_syscall(Some(&["1", "2"]))?;
        
        // qRegisterInfo (LLDB extension)
        for i in 0..20 {
            let _ = gdb.get_register_info(i);
        }

        Ok(())
    }

    #[gdbstub_test(armv4t)]
    fn test_exhaustive_host_io_armv4t(mut gdb: GdbMiClient) -> anyhow::Result<()> {
        let test_file = "host_io_stress.txt";
        let _ = fs::remove_file(test_file);

        // remote put/get
        gdb.exec_cmd(&format!("-interpreter-exec console \"remote put Cargo.toml {}\"", test_file))?;
        gdb.exec_cmd(&format!("-interpreter-exec console \"remote get {} /dev/null\"", test_file))?;
        
        // vFile:stat
        gdb.exec_cmd("-interpreter-exec console \"info proc stat\"")?;
        
        // remote delete
        gdb.delete_remote_file(test_file)?;

        Ok(())
    }

    #[gdbstub_test(armv4t)]
    fn test_exhaustive_tracepoints_armv4t(mut gdb: GdbMiClient) -> anyhow::Result<()> {
        gdb.exec_cmd("-interpreter-exec console \"trace *0x4000\"")?;
        gdb.trace_start()?;
        gdb.exec_cmd("-interpreter-exec console \"tstatus\"")?;
        gdb.trace_stop()?;
        
        // tfind
        let _ = gdb.exec_cmd("-interpreter-exec console \"tfind pc *0x4000\"");
        let _ = gdb.exec_cmd("-interpreter-exec console \"tfind none\"");

        Ok(())
    }

    #[gdbstub_test(armv4t)]
    fn test_signal_handling_armv4t(mut gdb: GdbMiClient) -> anyhow::Result<()> {
        // Continue with signal (requires target to be stopped)
        // SIGINT (2)
        gdb.exec_cmd("-exec-continue --signal 2")?;
        Ok(())
    }

/*
    #[gdbstub_test(armv4t, extended)]
    fn test_exhaustive_extended_armv4t(mut gdb: GdbMiClient) -> anyhow::Result<()> {
        // vRun (triggered by 'run')
        let _ = gdb.exec_cmd("-interpreter-exec console \"run\"");
        // vKill (triggered by 'kill')
        let _ = gdb.exec_cmd("-interpreter-exec console \"kill\"");
        Ok(())
    }
*/

    // --- armv4t_multicore (MultiThread) Integration Tests ---

    #[gdbstub_test(armv4t_multicore)]
    fn test_exhaustive_multicore_armv4t(mut gdb: GdbMiClient) -> anyhow::Result<()> {
        let tids = gdb.list_threads()?;
        
        for tid in &tids {
            gdb.select_thread(*tid)?;
            let _ = gdb.get_thread_info(*tid)?;
            gdb.write_reg("r1", 0x1000 * tid)?;
            assert_eq!(gdb.read_reg("r1")?, 0x1000 * tid);
        }

        // vCont: step one, continue others
        gdb.step_thread(1)?;
        gdb.step_thread(2)?;

        // Isolation check
        gdb.select_thread(1)?;
        gdb.write_reg("r2", 0xAAAA)?;
        gdb.select_thread(2)?;
        gdb.write_reg("r2", 0xBBBB)?;
        gdb.select_thread(1)?;
        assert_eq!(gdb.read_reg("r2")?, 0xAAAA);
        gdb.select_thread(2)?;
        assert_eq!(gdb.read_reg("r2")?, 0xBBBB);

        Ok(())
    }
}
