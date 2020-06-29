# gdbstub

[![](http://meritbadge.herokuapp.com/gdbstub)](https://crates.io/crates/gdbstub)
[![](https://docs.rs/gdbstub/badge.svg)](https://docs.rs/gdbstub)

An easy-to-use and easy-to-integrate implementation of the [GDB Remote Serial Protocol](https://sourceware.org/gdb/onlinedocs/gdb/Remote-Protocol.html#Remote-Protocol) in Rust.

`gdbstub` aims to provide a "drop-in" way to add GDB support to a project, _without_ requiring any large refactoring / ownership juggling. It is particularly useful in _emulators_, where it provides a powerful, non-intrusive way to debug code running within an emulated system. `gdbstub` is also _entirely `no_std`_, and can be run on bare-metal systems as well.

- [Documentation and Examples](https://docs.rs/gdbstub)

**Disclaimer:** `gdbstub` is still experiencing a fair amount of API churn! Expect (potentially large) breaking API changes between minor releases!

## Debugging Features

- Core GDB Protocol
    - Step + Continue
    - Add + Remove Software Breakpoints
    - Read/Write memory
    - Read/Write registers
    - (optional) Add + Remove Hardware Breakpoints
    - (optional) Read/Write/Access Watchpoints (i.e: value breakpoints)
- Extended GDB Protocol
    - (optional) Automatic architecture detection

Features marked as (optional) are not required to be implemented, but can provide an enhanced debugging experience if implemented.

The GDB Remote Serial Protocol is surprisingly complex, supporting advanced features such as remote file I/O, spawning new processes, "rewinding" program execution, and much, _much_ more. Thankfully, most of these features are completely optional, and getting a basic debugging session up-and-running only requires a small subset of commands to be implemented. Please open an issue / file a PR if `gdbstub` is missing a feature you'd like to use!

## Feature flags

`gdbstub` is `no_std` by default, though additional features can be enabled by toggling the `std` feature flag:

- Implements `Connection` for [`std::net::TcpStream`](https://doc.rust-lang.org/std/net/struct.TcpStream.html)
- Implements [`std::error::Error`](https://doc.rust-lang.org/std/error/trait.Error.html) for `gdbstub::Error`
- Outputs protocol responses via `log::trace!` (requires allocating a buffer for outgoing responses)

## Examples

The included `armv4t` example shows how `gdbstub` can be used to add `gdb` debugging support to a (incredibly simple) ARMv4T-based emulator. See it's `README.md` for details.

## Future Plans

- Improve multiprocess / multi-thread / multi-core support
- Improve packet-parsing infrastructure?
- Support addresses larger than 64-bits?
  - This would require plumbling-through the architecture's pointer size as a generic parameter into all the packet parsing code, which probably isn't _too_ difficult, just time consuming.

## Using `gdbstub` on bare-metal hardware

While the target use-case for `gdbstub` is emulation, the crate is entirely `no_std`, which means it _should_ be possible to implement a `gdbstub::Target` which uses low-level trap instructions + context switching to debug bare-metal code.

If you happen to stumble across this crate and use it to debug bare-metal hardware, please let me know!
