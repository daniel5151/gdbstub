//! A basic `no_std` example that's used to ballpark estimate how large
//! `gdbstub`'s binary footprint is resource-restricted environments.

#![no_std]
#![no_main]

use gdbstub::stub::state_machine::GdbStubStateMachine;
use gdbstub::stub::MultiThreadStopReason;
use gdbstub::stub::{DisconnectReason, GdbStubBuilder, GdbStubError};

mod conn;
mod gdb;
mod print_str;

use crate::print_str::print_str;

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo<'_>) -> ! {
    loop {}
}

fn rust_main() -> Result<(), i32> {
    print_str("Running example_no_std...");

    let mut target = gdb::DummyTarget::new();

    let conn = match conn::TcpConnection::new_localhost(9001) {
        Ok(c) => c,
        Err(e) => {
            print_str("could not start TCP server:");
            print_str(e);
            return Err(-1);
        }
    };

    let mut buf = [0; 4096];
    let gdb = GdbStubBuilder::new(conn)
        .with_packet_buffer(&mut buf)
        .build()
        .map_err(|_| 1)?;

    print_str("Starting GDB session...");

    let mut gdb = gdb.run_state_machine(&mut target).map_err(|_| 1)?;

    let res = loop {
        gdb = match gdb {
            GdbStubStateMachine::Idle(mut gdb) => {
                let byte = gdb.borrow_conn().read().map_err(|_| 1)?;
                match gdb.incoming_data(&mut target, byte) {
                    Ok(gdb) => gdb,
                    Err(e) => break Err(e),
                }
            }
            GdbStubStateMachine::Running(gdb) => {
                match gdb.report_stop(&mut target, MultiThreadStopReason::DoneStep) {
                    Ok(gdb) => gdb,
                    Err(e) => break Err(e),
                }
            }
            GdbStubStateMachine::CtrlCInterrupt(gdb) => {
                match gdb.interrupt_handled(&mut target, None::<MultiThreadStopReason<u32>>) {
                    Ok(gdb) => gdb,
                    Err(e) => break Err(e),
                }
            }
            GdbStubStateMachine::Disconnected(gdb) => break Ok(gdb.get_reason()),
        }
    };

    match res {
        Ok(disconnect_reason) => match disconnect_reason {
            DisconnectReason::Disconnect => print_str("GDB Disconnected"),
            DisconnectReason::TargetExited(_) => print_str("Target exited"),
            DisconnectReason::TargetTerminated(_) => print_str("Target halted"),
            DisconnectReason::Kill => print_str("GDB sent a kill command"),
        },
        Err(GdbStubError::TargetError(_e)) => {
            print_str("Target raised a fatal error");
        }
        Err(_e) => {
            print_str("gdbstub internal error");
        }
    }

    Ok(())
}

#[no_mangle]
extern "C" fn main(_argc: isize, _argv: *const *const u8) -> isize {
    if let Err(e) = rust_main() {
        return e as isize;
    }

    0
}
