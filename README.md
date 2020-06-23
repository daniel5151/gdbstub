# gdbstub

[![](http://meritbadge.herokuapp.com/gdbstub)](https://crates.io/crates/gdbstub)
[![](https://docs.rs/gdbstub/badge.svg)](https://docs.rs/gdbstub)

An implementation of the [GDB Remote Serial Protocol](https://sourceware.org/gdb/onlinedocs/gdb/Remote-Protocol.html#Remote-Protocol) in Rust.

`gdbstub` tries to make as few assumptions as possible about a project's architecture, and aims to provide a "drop-in" way to add GDB support, _without_ requiring any large refactoring / ownership juggling. It is particularly useful in _emulators_, where it provides a powerful, non-intrusive way to debug code running within an emulated system. `gdbstub` is also _entirely `no_std`_, and can be run on bare-metal systems as well.

- [Documentation and Examples](https://docs.rs/gdbstub)

**Disclaimer:** `gdbstub` is still in it's early stages of development! Expect breaking API changes between minor releases.

## Debugging Features

At the moment, `gdbstub` implements enough of the GDB Remote Serial Protocol to support step-through + breakpoint debugging of single-threaded code.

- Core GDB Protocol
    - Step + Continue
    - Add + Remove Breakpoints
    - Read/Write memory
    - Read/Write registers
    - (optional) Read/Write/Access Watchpoints (i.e: value breakpoints)
- Extended GDB Protocol
    - (optional) Automatic architecture detection

The GDB Remote Serial Protocol is surprisingly complex, supporting advanced features such as remote file I/O, spawning new processes, "rewinding" program execution, and much, _much_ more. Thankfully, most of these features are completely optional, and getting a basic debugging session up-and-running only requires a small subset of commands to be implemented.

## Feature flags

`gdbstub` is `no_std` by default.

Additional functionality can be enabled by activating certain features.

- `std` - (disabled by default)
  - Implements `Connection` for [`std::net::TcpStream`](https://doc.rust-lang.org/std/net/struct.TcpStream.html)
  - Implements [`std::error::Error`](https://doc.rust-lang.org/std/error/trait.Error.html) for `gdbstub::Error`
  - Outputs protocol responses via `log::trace!`

## Examples

The included `armv4t` example shows how `gdbstub` can be used to add `gdb` debugging support to a (incredibly simple) ARMv4T-based emulator. See it's `README.md` for details.

## Future Plans

- Improve packet-parsing infrastructure
    - Macros can be clever, but sometimes, they can be _too_ clever...
- Improve multiprocess / multi-thread / multi-core support?
- Support addresses larger than 64-bits?
  - This would require plumbling-through the achitecture's pointer size as a generic parameter into all the packet parsing code. It's probably not _too_ difficult, just time consuming.

## Using `gdbstub` on bare-metal hardware

While the target use-case for `gdbstub` is emulation, the crate is `no_std` compatible, which means it _should_ be possible to use `gdbstub` implement a `Target` which uses low-level trap instructions + context switching to debug bare-metal code.

If you happen to stumble across this crate and use it on bare-metal hardware, please let me know!
