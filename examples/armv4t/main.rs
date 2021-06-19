use std::net::{TcpListener, TcpStream};

#[cfg(unix)]
use std::os::unix::net::{UnixListener, UnixStream};

use gdbstub::state_machine::{Event, GdbStubStateMachine};
use gdbstub::target::ext::base::multithread::ThreadStopReason;
use gdbstub::{target::Target, ConnectionExt, DisconnectReason, GdbStub};

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

fn run_debugger<T: Target, C: ConnectionExt>(
    emu: &mut T,
    gdb: GdbStub<'_, T, C>,
) -> Result<DisconnectReason, gdbstub::GdbStubError<T::Error, C::Error>> {
    let mut gdb = gdb.run_state_machine()?;
    loop {
        gdb = match gdb {
            GdbStubStateMachine::Pump(mut gdb) => {
                let byte = gdb
                    .borrow_conn()
                    .read()
                    .map_err(gdbstub::GdbStubError::ConnectionRead)?;

                let (gdb, disconnect_reason) = gdb.pump(emu, byte)?;
                if let Some(disconnect_reason) = disconnect_reason {
                    break Ok(disconnect_reason);
                }
                gdb
            }

            GdbStubStateMachine::DeferredStopReason(mut gdb) => {
                // Note that the `armv4t` example doesn't use actually leverage deferred stop
                // reasons, and simply runs the emulator inline as part of the `resume` logic.
                //
                // Nonetheless, as a way to demonstrate the state-machine API, I've tweaked the
                // target's `resume` implementation to return `StopReason::Defer` if it peeks an
                // interrupt packet.
                //
                // An implementation that uses stop reasons would need to "select" on both the
                // data coming over the connection (which gets passed to `pump`) and whatever
                // mechanism it is using to detect stop events.
                //
                // The specifics of how this "select" mechanism might work will depends on where
                // `gdbstub` is being used.
                //
                // - A mpsc channel
                // - epoll/kqueue
                // - Running the target + stopping every so often to peek the connection
                // - Driving `GdbStub` from various interrupt handlers

                let byte = gdb
                    .borrow_conn()
                    .read()
                    .map_err(gdbstub::GdbStubError::ConnectionRead)?;

                let (gdb, event) = gdb.pump(emu, byte)?;
                match event {
                    Event::None => gdb,
                    Event::Disconnect(disconnect_reason) => break Ok(disconnect_reason),
                    Event::CtrlCInterrupt => {
                        // when an interrupt is received, report the `GdbInterrupt` stop reason.
                        if let GdbStubStateMachine::DeferredStopReason(gdb) = gdb {
                            match gdb.deferred_stop_reason(emu, ThreadStopReason::GdbInterrupt)? {
                                (_, Some(disconnect_reason)) => break Ok(disconnect_reason),
                                (gdb, None) => gdb,
                            }
                        } else {
                            gdb
                        }
                    }
                }
            }
        }
    }
}

fn main() -> DynResult<()> {
    pretty_env_logger::init();

    let mut emu = emu::Emu::new(TEST_PROGRAM_ELF)?;

    let connection: Box<dyn ConnectionExt<Error = std::io::Error>> = {
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

    let gdb = GdbStub::new(connection);

    match run_debugger(&mut emu, gdb) {
        Ok(disconnect_reason) => match disconnect_reason {
            DisconnectReason::Disconnect => {
                // run to completion
                while emu.step() != Some(emu::Event::Halted) {}
                let ret = emu.cpu.reg_get(armv4t_emu::Mode::User, 0);
                println!("Program completed. Return value: {}", ret)
            }
            DisconnectReason::TargetExited(code) => {
                println!("Target exited with code {}!", code)
            }
            DisconnectReason::TargetTerminated(sig) => {
                println!("Target terminated with signal {}!", sig)
            }
            DisconnectReason::Kill => println!("GDB sent a kill command!"),
        },
        Err(gdbstub::GdbStubError::TargetError(e)) => {
            println!("target encountered a fatal error: {}", e)
        }
        Err(e) => {
            println!("gdbstub encountered a fatal error: {}", e)
        }
    }

    Ok(())
}
