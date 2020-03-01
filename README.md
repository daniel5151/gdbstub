# gdbstub

An implementation of the [GDB Remote Server Protocol](https://sourceware.org/gdb/onlinedocs/gdb/Remote-Protocol.html#Remote-Protocol) in Rust, primarily for use in _emulators_.

`gdbstub` tries to make as few assumptions as possible about your emulator's architecture, and aims to provide a "drop-in" way to add GDB support into an emulator, _without_ any large refactoring / ownership juggling (hopefully).

## Debugging Features

- Core Protocol
    - [x] Step / Continue
    - [x] Add / Remove Breakpoints
    - [x] Read/Write memory
    - [x] Read/Write registers
    - [ ] Read/Write/Access Watchpoints (i.e: value breakpoints)
      - implemented, but currently broken
- Extended Protocol
    - [x] Automatic architecture detection (via target.xml)

There are also a few features which rely on `std`, which can be enabled by enabling the `std` feature:

- An `impl Connection` for some common std types (notably: TcpStream)
- Additional logging (outputs protocol responses via `trace!`)

## Future Plans

- Improve packet-parsing infrastructure
- Improve multiprocess / multithread / multicore support?
- Re-architect internals to remove `alloc` dependency?
  - Current approach has a clear separation between packet parsing and command execution, and requires intermediate allocations for parsed data. Interleaving packet parsing and command execution would remove the need for intermediate allocations, at the expense of potentially less-clear code...
  - Require users to allocate packet buffers themselves

## Example

_Disclaimer:_ `gdbstub`'s API and architecture is still very much in flux, so expect things to change often and "destructively"

This snippet provides a _very brief_ overview of how to use `gdbstub`.

While I have a few projects which are already using gdbstub, none of them are open-source (at the moment). In the future, I'll try to find some time time to create a more robust (read: compiling) example.

```rust
use std::net::{TcpListener, TcpStream};

use gdbstub::{GdbStub, Access, AccessKind, Target, TargetState};

// <your pre-existing emulator>
struct MySystem { /* ... */ }

// `Target` is the fundamental trait of `gdbstub`, wrapping the multitude of different
// emulator implementations behind a single, generic interface which the GdbStub can
// query for information / drive forward in response to incoming GDB commands.
impl Target for MySystem {
    // The target's pointer size
    type Usize = u32;
    // A user-defined error type (for passing-through any internal emulation errors)
    type Error = ();

    // Run the system for a single "step", using the provided callback to log
    // any memory accesses which may have occurred
    fn step(
        &mut self,
        mut log_mem_access: impl FnMut(Access<u32>),
    ) -> Result<TargetState, Self::Error> {
        self.cpu.cycle()?;

        for (read_or_write, addr, val) in self.mem.recent_accesses.drain(..) {
            log_mem_access(Access {
                kind: if read_or_write {
                    AccessKind::Read
                } else {
                    AccessKind::Write
                },
                addr,
                val
            })
        }

        Ok(TargetState::Running)
    }

    // Read-out the CPU's register values in the order specified in the arch's
    // `target.xml` file.
    // e.g: for ARM: binutils-gdb/blob/master/gdb/features/arm/arm-core.xml
    fn read_registers(&mut self, mut push_reg: impl FnMut(&[u8])) {
        // general purpose registers
        for i in 0..13 {
            push_reg(&self.cpu.reg_get(i).to_le_bytes());
        }
        push_reg(&self.cpu.reg_get(reg::SP).to_le_bytes());
        push_reg(&self.cpu.reg_get(reg::LR).to_le_bytes());
        push_reg(&self.cpu.reg_get(reg::PC).to_le_bytes());
        // Floating point registers, unused
        for _ in 0..25 {
            push_reg(&[0, 0, 0, 0]);
        }
        push_reg(&self.cpu.reg_get(reg::CPSR).to_le_bytes());
    }

    fn read_pc(&mut self) -> u32 {
        self.cpu.reg_get(reg::PC)
    }

    // read the specified memory addresses from the target
    fn read_addrs(&mut self, addr: std::ops::Range<u32>, mut push_byte: impl FnMut(u8)) {
        for addr in addr {
            push_byte(self.mem.r8(addr))
        }
    }

    // write data to the specified memory addresses
    fn write_addrs(&mut self, mut get_addr_val: impl FnMut() -> Option<(u32, u8)>) {
        while let Some((addr, val)) = get_addr_val() {
            self.mem.w8(addr, val);
        }
    }

    // there are a few other optional methods which can be implemented to enable
    // some more advanced functionality (e.g: automatic arch detection).
    // See the docs for details.
}


fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Your existing setup code...
    let mut system = MySystem::new()?;
    // ...

    // e.g: using a TcpStream for the GDB connection
    let sockaddr = format!("localhost:{}", 9001);
    eprintln!("Waiting for a GDB connection on {:?}...", sockaddr);
    let sock = TcpListener::bind(sockaddr)?;
    let (stream, addr) = sock.accept()?;
    eprintln!("Debugger connected from {}", addr);

    // At this point, it's possible to connect to the emulator using
    // `gdb-multiarch -iex "target remote localhost:9001"`

    // Hand the connection off to the GdbStub.
    let debugger = GdbStub::new(stream);

    // Instead of taking ownership of the system, GdbStub takes a &mut, yielding ownership once the debugging session is closed, or an error occurs.
    let system_result = match debugger.run(&mut system) {
        Ok(state) => {
            eprintln!("Disconnected from GDB. Target state: {:?}", state);
            Ok(())
        }
        Err(gdbstub::Error::TargetError(e)) => Err(e),
        Err(e) => return Err(e.into()),
    };

    eprintln!("{:?}", system_result);
}


```

## Using `gdbstub` on actual hardware

While the target use-case for `gdbstub` is emulation, the crate is `no_std` compatible (albeit with a dependency on `alloc`), which means it _should_ be possible to use in embedded contexts as well.

At the moment, this is not a supported use-case, and has not been tested. Please let me know if you've had any success using `gdbstub` on actual hardware!
