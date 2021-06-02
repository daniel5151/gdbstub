#![no_std]
#![no_main]

extern crate libc;

use gdbstub::{Connection, DisconnectReason, GdbStubBuilder, GdbStubError};

mod conn;
mod gdb;
mod print_str;

use crate::print_str::print_str;

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

fn rust_main() -> Result<(), i32> {
    // pretty_env_logger::init();

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

    let (mut gdb, mut state) = gdb.run_state_machine().map_err(|_| 1)?;

    loop {
        use gdbstub::state_machine::State;

        state = match state {
            State::Pump(token) => {
                let byte = gdb.borrow_conn().read().map_err(|_| 1)?;
                match gdb.pump(token, &mut target, byte) {
                    Ok(next_state) => next_state,
                    Err(GdbStubError::TargetError(_e)) => {
                        print_str("Target raised a fatal error");
                        break;
                    }
                    Err(_e) => {
                        print_str("gdbstub internal error");
                        break;
                    }
                }
            }
            State::Disconnect(disconnect_reason, token) => {
                match disconnect_reason {
                    DisconnectReason::Disconnect => {
                        print_str("GDB Disconnected");
                        break;
                    }
                    DisconnectReason::TargetExited(_) => print_str("Target exited"),
                    DisconnectReason::TargetTerminated(_) => print_str("Target halted"),
                    DisconnectReason::Kill => print_str("GDB sent a kill command"),
                };
                token.into_state()
            }
            // example_no_std stubs out resume, so this will never happen
            State::DeferredStopReason(_) => return Err(-1),
        }
    }

    Ok(())
}

#[no_mangle]
pub extern "C" fn main(_argc: isize, _argv: *const *const u8) -> isize {
    if let Err(e) = rust_main() {
        return e as isize;
    }

    0
}
