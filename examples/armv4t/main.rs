use std::net::{TcpListener, TcpStream};

pub type DynResult<T> = Result<T, Box<dyn std::error::Error>>;

static TEST_PROGRAM_ELF: &[u8] = include_bytes!("test_bin/test.elf");

mod emu;
mod gdb;
mod mem_sniffer;

fn new_tcp_gdbstub<'a, T>(
    port: u16,
    buf: &'a mut [u8],
) -> DynResult<gdbstub::GdbStub<'a, T, TcpStream>>
where
    T: gdbstub::Target,
{
    let sockaddr = format!("127.0.0.1:{}", port);
    eprintln!("Waiting for a GDB connection on {:?}...", sockaddr);

    let sock = TcpListener::bind(sockaddr)?;
    let (stream, addr) = sock.accept()?;
    eprintln!("Debugger connected from {}", addr);

    Ok(gdbstub::GdbStub::new(stream, buf))
}

fn main() -> DynResult<()> {
    pretty_env_logger::init();

    let mut emu = emu::Emu::new(TEST_PROGRAM_ELF)?;

    // hook-up debugger
    let mut pktbuf = [0; 4096];
    let mut debugger = new_tcp_gdbstub(9001, &mut pktbuf)?;

    let state_after_disconnect = debugger.run(&mut emu)?;
    if state_after_disconnect == gdbstub::TargetState::Running {
        // run to completion
        while emu.step() != Some(emu::Event::Halted) {}
    }

    let ret = emu.cpu.reg_get(armv4t_emu::Mode::User, 0);
    println!("Program completed. Return value: {}", ret);

    Ok(())
}
