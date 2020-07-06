# gdbstub

[![](http://meritbadge.herokuapp.com/gdbstub)](https://crates.io/crates/gdbstub)
[![](https://docs.rs/gdbstub/badge.svg)](https://docs.rs/gdbstub)

An easy-to-use and easy-to-integrate implementation of the [GDB Remote Serial Protocol](https://sourceware.org/gdb/onlinedocs/gdb/Remote-Protocol.html#Remote-Protocol) in Rust.

`gdbstub` is particularly useful in emulators, where it provides a powerful, non-intrusive way to debug code running within an emulated system. The API aims to provide a "drop-in" way to add GDB support to an existing project, without requiring any large refactoring / ownership juggling.

`gdbstub` is also `no_std` compatible, and can be used on platforms without a global allocator (`alloc`). In embedded contexts, `gdbstub` can be configured to use pre-allocated buffers and communicate over any serial I/O connection (e.g: UART).

- [Documentation and Examples](https://docs.rs/gdbstub)

**Warning:** `gdbstub` is still undergoing a fair amount of API churn. Beware when upgrading between minor releases!

## Debugging Features

Features marked as (optional) are not required to be implemented, but can provide an enhanced debugging experience if implemented.

- Core GDB Protocol
    - Step + Continue
    - Add + Remove Software Breakpoints
    - Read/Write memory
    - Read/Write registers
    - (optional) Add + Remove Hardware Breakpoints
    - (optional) Read/Write/Access Watchpoints (i.e: value breakpoints)
    - (optional) Multithreading support
- Extended GDB Protocol
    - (optional) Handle custom commands sent via GDB's `monitor` command
    - (optional) Automatic architecture detection

The GDB Remote Serial Protocol is surprisingly complex, supporting advanced features such as remote file I/O, spawning new processes, "rewinding" program execution, and much, _much_ more. Thankfully, most of these features are completely optional, and getting a basic debugging session up-and-running only requires a small subset of commands to be implemented. Please open an issue / file a PR if `gdbstub` is missing a feature you'd like to use!

## Feature flags

`gdbstub` enables the `std` feature by default. In `no_std` contexts, use `default-features = false`.

- `alloc`
    - Implements `Connection` for `Box<dyn Connection>`
- `std` (implies `alloc`)
    - Implement `Connection` for [`TcpStream`](https://doc.rust-lang.org/std/net/struct.TcpStream.html) and [`UnixStream`](https://doc.rust-lang.org/std/os/unix/net/struct.UnixStream.html).
    - Implement [`std::error::Error`](https://doc.rust-lang.org/std/error/trait.Error.html) for `gdbstub::Error`
    - Log outgoing packets via `log::trace!` (uses a heap-allocated output buffer)

## Examples

### `armv4t`

The `armv4t` example shows how `gdbstub` can be used to add `gdb` debugging support to an (incredibly simple) ARMv4T-based emulator. See `examples/armv4t/README.md` for details.

### `armv4t_multicore`

A dual-core variation of the `armv4t` example. Implements `gdbstub`'s multithread extensions to enable per-core debugging. See `examples/armv4t_multicore/README.md` for details.

## Real-World Examples

There are already several projects which use `gdbstub`:

- [rustyboyadvance-ng](https://github.com/michelhe/rustboyadvance-ng/) - Nintendo GameBoy Advance emulator and debugger
- [microcorruption-emu](https://github.com/sapir/microcorruption-emu) - msp430 emulator for the microcorruption.com ctf
- [clicky](https://github.com/daniel5151/clicky/) - An emulator for classic clickwheel iPods (dual-core ARMv4T SoC)
- [ts7200](https://github.com/daniel5151/ts7200/) - An emulator for the TS-7200, a somewhat bespoke embedded ARMv4t platform

## Future Plans

- Improve multiprocess / multi-thread / multi-core support
    - Support thread-specific breakpoints
    - Support non-stop mode?
    - Support disabling multiprocess extensions on older GDB clients
- Support addresses larger than 64-bits?
  - This would require plumbing-through the architecture's pointer size as a generic parameter into all the packet parsing code, which probably isn't _too_ difficult, just time consuming.

## Using `gdbstub` on bare-metal hardware

Since `gdbstub` is `no_std` compatible, it should be possible to implement a `gdbstub::Target` which uses low-level trap instructions + context switching to debug bare-metal code.

If you happen to stumble across this crate and end up using it to debug some bare-metal code, please let me know! I'd love to link to your project!
