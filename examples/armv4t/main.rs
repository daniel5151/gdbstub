//! An incredibly simple emulator to run elf binaries compiled with
//! `arm-none-eabi-cc -march=armv4t`. It's not modeled after any real-world
//! system.

use gdbstub::common::Signal;
use gdbstub::conn::Connection;
use gdbstub::conn::ConnectionExt;
use gdbstub::stub::run_blocking;
use gdbstub::stub::DisconnectReason;
use gdbstub::stub::GdbStub;
use gdbstub::stub::SingleThreadStopReason;
use gdbstub::target::Target;
use std::net::TcpListener;
use std::net::TcpStream;
#[cfg(unix)]
use std::os::unix::net::UnixListener;
#[cfg(unix)]
use std::os::unix::net::UnixStream;

type DynResult<T> = Result<T, Box<dyn std::error::Error>>;

const TEST_PROGRAM_ELF: &[u8] = include_bytes!("test_bin/test.elf");

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

enum EmuGdbEventLoop {}

impl run_blocking::BlockingEventLoop for EmuGdbEventLoop {
    type Target = emu::Emu;
    type Connection = Box<dyn ConnectionExt<Error = std::io::Error>>;
    type StopReason = SingleThreadStopReason<u32>;

    #[allow(clippy::type_complexity)]
    fn wait_for_stop_reason(
        target: &mut emu::Emu,
        conn: &mut Self::Connection,
    ) -> Result<
        run_blocking::Event<SingleThreadStopReason<u32>>,
        run_blocking::WaitForStopReasonError<
            <Self::Target as Target>::Error,
            <Self::Connection as Connection>::Error,
        >,
    > {
        // The `armv4t` example runs the emulator in the same thread as the GDB state
        // machine loop. As such, it uses a simple poll-based model to check for
        // interrupt events, whereby the emulator will check if there is any incoming
        // data over the connection, and pause execution with a synthetic
        // `RunEvent::IncomingData` event.
        //
        // In more complex integrations, the target will probably be running in a
        // separate thread, and instead of using a poll-based model to check for
        // incoming data, you'll want to use some kind of "select" based model to
        // simultaneously wait for incoming GDB data coming over the connection, along
        // with any target-reported stop events.
        //
        // The specifics of how this "select" mechanism work + how the target reports
        // stop events will entirely depend on your project's architecture.
        //
        // Some ideas on how to implement this `select` mechanism:
        //
        // - A mpsc channel
        // - epoll/kqueue
        // - Running the target + stopping every so often to peek the connection
        // - Driving `GdbStub` from various interrupt handlers

        let poll_incoming_data = || {
            // gdbstub takes ownership of the underlying connection, so the `borrow_conn`
            // method is used to borrow the underlying connection back from the stub to
            // check for incoming data.
            conn.peek().map(|b| b.is_some()).unwrap_or(true)
        };

        match target.run(poll_incoming_data) {
            emu::RunEvent::IncomingData => {
                let byte = conn
                    .read()
                    .map_err(run_blocking::WaitForStopReasonError::Connection)?;
                Ok(run_blocking::Event::IncomingData(byte))
            }
            emu::RunEvent::Event(event) => {
                use gdbstub::target::ext::breakpoints::WatchKind;

                // translate emulator stop reason into GDB stop reason
                let stop_reason = match event {
                    emu::Event::DoneStep => SingleThreadStopReason::DoneStep,
                    emu::Event::Halted => SingleThreadStopReason::Terminated(Signal::SIGSTOP),
                    emu::Event::Break => SingleThreadStopReason::SwBreak(()),
                    emu::Event::WatchWrite(addr) => SingleThreadStopReason::Watch {
                        tid: (),
                        kind: WatchKind::Write,
                        addr,
                    },
                    emu::Event::WatchRead(addr) => SingleThreadStopReason::Watch {
                        tid: (),
                        kind: WatchKind::Read,
                        addr,
                    },
                };

                Ok(run_blocking::Event::TargetStopped(stop_reason))
            }
        }
    }

    fn on_interrupt(
        _target: &mut emu::Emu,
    ) -> Result<Option<SingleThreadStopReason<u32>>, <emu::Emu as Target>::Error> {
        // Because this emulator runs as part of the GDB stub loop, there isn't any
        // special action that needs to be taken to interrupt the underlying target. It
        // is implicitly paused whenever the stub isn't within the
        // `wait_for_stop_reason` callback.
        Ok(Some(SingleThreadStopReason::Signal(Signal::SIGINT)))
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

    match gdb.run_blocking::<EmuGdbEventLoop>(&mut emu) {
        Ok(disconnect_reason) => match disconnect_reason {
            DisconnectReason::Disconnect => {
                println!("GDB client has disconnected. Running to completion...");
                while emu.step() != Some(emu::Event::Halted) {}
            }
            DisconnectReason::TargetExited(code) => {
                println!("Target exited with code {}!", code)
            }
            DisconnectReason::TargetTerminated(sig) => {
                println!("Target terminated with signal {}!", sig)
            }
            DisconnectReason::Kill => println!("GDB sent a kill command!"),
        },
        Err(e) => {
            if e.is_target_error() {
                println!(
                    "target encountered a fatal error: {}",
                    e.into_target_error().unwrap()
                )
            } else if e.is_connection_error() {
                let (e, kind) = e.into_connection_error().unwrap();
                println!("connection error: {:?} - {}", kind, e,)
            } else {
                println!("gdbstub encountered a fatal error: {}", e)
            }
        }
    }

    let ret = emu.cpu.reg_get(armv4t_emu::Mode::User, 0);
    println!("Program completed. Return value: {}", ret);

    Ok(())
}
