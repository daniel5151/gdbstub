# gdbstub

[![](http://meritbadge.herokuapp.com/gdbstub)](https://crates.io/crates/gdbstub)
[![](https://docs.rs/gdbstub/badge.svg)](https://docs.rs/gdbstub)

An easy-to-use and easy-to-integrate implementation of the [GDB Remote Serial Protocol](https://sourceware.org/gdb/onlinedocs/gdb/Remote-Protocol.html#Remote-Protocol) in Rust.

`gdbstub` is particularly useful in emulators, where it provides a powerful, non-intrusive way to debug code running within an emulated system. The API aims to provide a "drop-in" way to add GDB support to an existing project, without requiring any large refactoring / ownership juggling.

`gdbstub` is also entirely `no_std`, _without_ a dependency on `alloc`! If you're interested in adding remote debugging support to a resource-constrained bare-metal system, give `gdbstub` a shot!

- [Documentation and Examples](https://docs.rs/gdbstub)

**Warning:** `gdbstub` is still experiencing a fair amount of API churn, so expect breaking API changes between minor releases!

## Debugging Features

- Core GDB Protocol
    - Step + Continue
    - Add + Remove Software Breakpoints
    - Read/Write memory
    - Read/Write registers
    - (optional) Add + Remove Hardware Breakpoints
    - (optional) Read/Write/Access Watchpoints (i.e: value breakpoints)
- Extended GDB Protocol
    - (optional) Support custom GDB commands sent via `monitor`
    - (optional) Automatic architecture detection

Features marked as (optional) are not required to be implemented, but can provide an enhanced debugging experience if implemented.

The GDB Remote Serial Protocol is surprisingly complex, supporting advanced features such as remote file I/O, spawning new processes, "rewinding" program execution, and much, _much_ more. Thankfully, most of these features are completely optional, and getting a basic debugging session up-and-running only requires a small subset of commands to be implemented. Please open an issue / file a PR if `gdbstub` is missing a feature you'd like to use!

## Feature flags

`gdbstub` is `no_std` by default, though additional features can be enabled by toggling various feature flags:

- `alloc`
    - Implements `Connection` for `Box<dyn Connection>`
    - Log outgoing packets via `log::trace!` (using heap-allocated output buffer)
- `std` (implies `alloc`)
    - Implement `Connection` for [`TcpStream`](https://doc.rust-lang.org/std/net/struct.TcpStream.html) and [`UnixStream`](https://doc.rust-lang.org/std/os/unix/net/struct.UnixStream.html).
    - Implement [`std::error::Error`](https://doc.rust-lang.org/std/error/trait.Error.html) for `gdbstub::Error`

## Examples

The included `armv4t` example shows how `gdbstub` can be used to add `gdb` debugging support to a (incredibly simple) ARMv4T-based emulator. See it's `README.md` for details.

## Future Plans

- Improve multiprocess / multi-thread / multi-core support
- Improve packet-parsing infrastructure?
- Support addresses larger than 64-bits?
  - This would require plumbing-through the architecture's pointer size as a generic parameter into all the packet parsing code, which probably isn't _too_ difficult, just time consuming.

## Using `gdbstub` on bare-metal hardware

Since `gdbstub` is entirely `no_std`, it _should_ be possible to implement a `gdbstub::Target` which uses low-level trap instructions + context switching to debug bare-metal code.

If you happen to stumble across this crate and use it to debug bare-metal hardware, please let me know! I'd love to link to your project!
