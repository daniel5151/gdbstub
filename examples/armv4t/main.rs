use std::net::{TcpListener, TcpStream};

#[cfg(unix)]
use std::os::unix::net::{UnixListener, UnixStream};

use gdbstub::{Connection, DisconnectReason, GdbStub};

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
    let mut pktbuf = [0; 4096];
    let mut debugger = GdbStub::new(connection, &mut pktbuf);

    match debugger.run(&mut emu)? {
        DisconnectReason::Disconnect => {
            // run to completion
            while emu.step() != Some(emu::Event::Halted) {}
        }
        DisconnectReason::TargetHalted => println!("Target halted!"),
        DisconnectReason::Kill => {
            println!("GDB sent a kill command!");
            return Ok(());
        }
    }

    let ret = emu.cpu.reg_get(armv4t_emu::Mode::User, 0);
    println!("Program completed. Return value: {}", ret);

    Ok(())
}
