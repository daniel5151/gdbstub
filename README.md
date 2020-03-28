# gdbstub

[![](http://meritbadge.herokuapp.com/gdbstub)](https://crates.io/crates/gdbstub)
[![](https://docs.rs/gdbstub/badge.svg)](https://docs.rs/gdbstub)

An implementation of the [GDB Remote Serial Protocol](https://sourceware.org/gdb/onlinedocs/gdb/Remote-Protocol.html#Remote-Protocol) in Rust, primarily for use in emulators.

`gdbstub` tries to make as few assumptions as possible about a project's architecture, and aims to provide a "drop-in" way to add GDB support, _without_ requiring any large refactoring / ownership juggling. It is particularly useful in _emulators_, where it provides a powerful, non-intrusive way to debug code running within an emulated system.

- [Documentation and Examples](https://docs.rs/gdbstub)

**Disclaimer:** `gdbstub` is still in it's early stages of development! Expect breaking API changes between minor releases.

## Debugging Features

At the moment, `gdbstub` implements enough of the GDB Remote Serial Protocol to support step-through + breakpoint debugging of single-threaded code.

- Core GDB Protocol
    - Step + Continue
    - Add + Remove Breakpoints
    - Read/Write memory
    - Read/Write registers
    - Read/Write/Access Watchpoints (i.e: value breakpoints) (_currently broken_)
- Extended GDB Protocol
    - (optional) Automatic architecture detection

The GDB Remote Serial Protocol is surprisingly complex, supporting advanced features such as remote file I/O, spawning new processes, "rewinding" program execution, and much, _much_ more. Thankfully, most of these features are completely optional, and getting a basic debugging session up-and-running only requires a small subset of commands to be implemented.

## Feature flags

`gdbstub` is `no_std` by default, though it does have a dependency on `alloc`.

Additional functionality can be enabled by activating certain features.

- `std` - (disabled by default)
  - Implements `Connection` for [`std::net::TcpStream`](https://doc.rust-lang.org/std/net/struct.TcpStream.html)
  - Implements [`std::error::Error`](https://doc.rust-lang.org/std/error/trait.Error.html) for `gdbstub::Error`
  - Outputs protocol responses via `log::trace!`

## Future Plans

- Improve packet-parsing infrastructure
    - Macros can be clever, but sometimes, they can be _too_ clever...
- Improve multiprocess / multi-thread / multi-core support?
- Re-architect internals to remove `alloc` dependency (for lower-end embedded targets)
  - The current `gdbstub` implementation clearly separates packet parsing and command execution, and uses intermediate allocations to store structured command data. Interleaving packet parsing and command execution would remove the need for these intermediate allocations, at the expense of potentially less clear code...
  - Would require users to allocate packet buffers themselves

## Using `gdbstub` on bare-metal hardware

While the target use-case for `gdbstub` is emulation, the crate is `no_std` compatible (albeit with a dependency on `alloc`), which means it _should_ be possible to use in embedded contexts as well.

At the moment, this is not a "first-class" use-case, and has not been tested. Please let me know if you've had any success using `gdbstub` on actual hardware!
