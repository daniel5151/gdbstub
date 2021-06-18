use std::net::{TcpListener, TcpStream};

#[cfg(unix)]
use std::os::unix::net::{UnixListener, UnixStream};

use gdbstub::{state_machine::GdbStubStateMachine, Connection, DisconnectReason, GdbStub};

pub type DynResult<T> = Result<T, Box<dyn std::error::Error>>;

static TEST_PROGRAM_ELF: &[u8] = include_bytes!("test_bin/test.elf");

mod emu;
mod gdb;
mod mem_sniffer;

fn wait_for_tcp(port: u16) -> DynResult<TcpStream> {
    let sockaddr = format!("127.0.0.1:{}", port);
    eprintln!("Waiting for a GDB connection on {:?}...", sockaddr);

    let sock = TcpListener::bind(sockaddr)?;
    let (stream, addr) = sock.accept()?;
    eprintln!("Debugger connected from {}", addr);

    Ok(stream)
}

#[cfg(unix)]
fn wait_for_uds(path: &str) -> DynResult<UnixStream> {
    match std::fs::remove_file(path) {
        Ok(_) => {}
        Err(e) => match e.kind() {
            std::io::ErrorKind::NotFound => {}
            _ => return Err(e.into()),
        },
    }

    eprintln!("Waiting for a GDB connection on {}...", path);

    let sock = UnixListener::bind(path)?;
    let (stream, addr) = sock.accept()?;
    eprintln!("Debugger connected from {:?}", addr);

    Ok(stream)
}

fn main() -> DynResult<()> {
    pretty_env_logger::init();

    let mut emu = emu::Emu::new(TEST_PROGRAM_ELF)?;

    let connection: Box<dyn Connection<Error = std::io::Error>> = {
        if std::env::args().nth(1) == Some("--uds".to_string()) {
            #[cfg(not(unix))]
            {
                return Err("Unix Domain Sockets can only be used on Unix".into());
            }
            #[cfg(unix)]
            {
                Box::new(wait_for_uds("/tmp/armv4t_gdb")?)
            }
        } else {
            Box::new(wait_for_tcp(9001)?)
        }
    };

    // hook-up debugger
    let gdb = GdbStub::new(connection);
    let mut gdb = gdb.run_state_machine()?;
    loop {
        gdb = match gdb {
            GdbStubStateMachine::Pump(mut gdb) => {
                let byte = gdb.borrow_conn().read()?;
                match gdb.pump(&mut emu, byte) {
                    Ok((_, Some(disconnect_reason))) => {
                        match disconnect_reason {
                            DisconnectReason::Disconnect => {
                                // run to completion
                                while emu.step() != Some(emu::Event::Halted) {}
                            }
                            DisconnectReason::TargetExited(code) => {
                                println!("Target exited with code {}!", code)
                            }
                            DisconnectReason::TargetTerminated(sig) => {
                                println!("Target terminated with signal {}!", sig)
                            }
                            DisconnectReason::Kill => println!("GDB sent a kill command!"),
                        }
                        break;
                    }
                    Ok((gdb, None)) => gdb,
                    Err(gdbstub::GdbStubError::TargetError(_e)) => {
                        println!("Target raised a fatal error");
                        break;
                    }
                    Err(e) => {
                        println!("gdbstub internal error: {}", e);
                        break;
                    }
                }
            }

            GdbStubStateMachine::DeferredStopReason(mut gdb) => {
                // armv4t example doesn't actually defer stop reasons
                // instead, i've wired it up to defer GDB interrupts, as a way to demonstrate
                // the API
                //
                // in a system with proper deferred stop reasons, you'll have to "select" on
                // both the incoming data, and whatever mechanism you're using to detect stop
                // events.
                //
                // I will think about how to improve the API to avoid having this manual byte !=
                // 0x03 check, because this is pretty ugly at the moment.

                let byte = gdb.borrow_conn().read()?;
                if byte != 0x03 {
                    println!("expected breakpoint packet, got something else: {}", byte);
                    return Ok(());
                }

                eprintln!("deferred_stop_reason with GdbInterrupt");

                match gdb.deferred_stop_reason(
                    &mut emu,
                    gdbstub::target::ext::base::multithread::ThreadStopReason::GdbInterrupt,
                ) {
                    Ok((_, Some(disconnect_reason))) => {
                        match disconnect_reason {
                            DisconnectReason::Disconnect => {
                                // run to completion
                                while emu.step() != Some(emu::Event::Halted) {}
                            }
                            DisconnectReason::TargetExited(code) => {
                                println!("Target exited with code {}!", code)
                            }
                            DisconnectReason::TargetTerminated(sig) => {
                                println!("Target terminated with signal {}!", sig)
                            }
                            DisconnectReason::Kill => println!("GDB sent a kill command!"),
                        }
                        break;
                    }
                    Ok((gdb, None)) => gdb,
                    Err(gdbstub::GdbStubError::TargetError(_e)) => {
                        println!("Target raised a fatal error");
                        break;
                    }
                    Err(e) => {
                        println!("gdbstub internal error: {}", e);
                        break;
                    }
                }
            }
        }
    }

    let ret = emu.cpu.reg_get(armv4t_emu::Mode::User, 0);
    println!("Program completed. Return value: {}", ret);

    Ok(())
}
