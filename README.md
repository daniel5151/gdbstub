# gdbstub

[![](http://meritbadge.herokuapp.com/gdbstub)](https://crates.io/crates/gdbstub)
[![](https://docs.rs/gdbstub/badge.svg)](https://docs.rs/gdbstub)

> NOTE: gdbstub's master branch is currently preparing breaking changes. 
> For the most recently released code, look to the `0.3.0` tag.

An ergonomic and easy-to-integrate implementation of the [GDB Remote Serial Protocol](https://sourceware.org/gdb/onlinedocs/gdb/Remote-Protocol.html#Remote-Protocol) in Rust.

Why `gdbstub`?

-   **Excellent Ergonomics**
    -   Unlike other GDB stub libraries, which simply expose the underlying GDB protocol "warts and all", `gdbstub` tries to abstract as much of the raw GDB protocol details from the user.
        -   For example, instead of having to dig through some [obscure XML files deep the GDB codebase](https://github.com/bminor/binutils-gdb/tree/master/gdb/features) just to read/write from CPU registers, `gdbstub` comes with [built-in register definitions](https://docs.rs/gdbstub/*/gdbstub/arch/index.html) for most common architectures!
    -   `gdbstub` makes _extensive_ use of Rust's powerful type system + generics to enforce protocol invariants at compile time, minimizing the number of tricky protocol details end users have to worry about.
-   **Easy to Integrate**
    -   `gdbstub`'s API is designed to be as unobtrusive as possible, and shouldn't require any large refactoring effort to integrate into an existing project. It doesn't require taking direct ownership of any key data structures,
-   **`#![no_std]` Ready & Size Optimized**
    -   Can be configured to use fixed-size, pre-allocated buffers. **`gdbstub` does _not_ depend on `alloc`.**
    -   `gdbstub` is transport-layer agnostic, and uses an abstract [`Connection`](https://docs.rs/gdbstub/*/gdbstub/trait.Connection.html) interface to communicate with the GDB server. As long as target has some method of performing in-order, serial, byte-wise I/O (e.g: UART), it's possible to run `gdbstub` on it.
    -   "You don't pay for what you don't use": If you don't implement a particular protocol extension, the resulting binary won't include _any_ code related to parsing/handling that extension's packets! See the [Zero-overhead Protocol Extensions](#zero-overhead-protocol-extensions) section below for more details.
    -   A lot of work has gone into reducing `gdbstub`'s binary and RAM footprints.
        -   In release builds, using all the tricks in the [`min-sized-rust`](https://github.com/johnthagen/min-sized-rust) guidelines, a baseline `gdbstub` implementation weighs in at roughly **_10kb of `.text` and negligible `.rodata`!_** \*
        -   This is already pretty good, and I suspect that there are still lots of low-hanging optimizations which can reduce the size even further.

\* Exact numbers vary by target platform, compiler version, and `gdbstub` revision. Data was collected using the included `example_no_std` project + [`cargo bloat`](https://github.com/RazrFalcon/cargo-bloat).

`gdbstub` is particularly well suited for _emulation_, making it easy to add powerful, non-intrusive debugging support to an emulated system. Just provide an implementation of the [`Target`](https://docs.rs/gdbstub/*/gdbstub/target/trait.Target.html) trait for your target platform, and you're basically ready to start debugging!

-   [Documentation](https://docs.rs/gdbstub)

## Debugging Features

Features marked as (optional) aren't required to be implemented, but can be implemented to enhance the debugging experience.

-   Core GDB Protocol
    -   Step + Continue
    -   Add + Remove Software Breakpoints
    -   Read/Write memory
    -   Read/Write registers
    -   (optional) Add + Remove Hardware Breakpoints
    -   (optional) Read/Write/Access Watchpoints (i.e: value breakpoints)
    -   (optional) Multithreading support
-   Extended GDB Protocol
    -   (optional) Handle custom debug commands (sent via GDB's `monitor` command)
    -   (optional) Automatic architecture detection

The GDB Remote Serial Protocol is surprisingly complex, supporting advanced features such as remote file I/O, spawning new processes, "rewinding" program execution, and much, _much_ more. Thankfully, most of these features are completely optional, and getting a basic debugging session up-and-running only requires a small subset of commands to be implemented.

If `gdbstub` is missing a feature you'd like to use, please file an issue / open a PR!

### Zero-overhead Protocol Extensions

Using a novel technique called **Inlineable Dyn Extension Traits** (IDETs), `gdbstub` is able to leverage the Rust compiler's powerful optimization passes to ensure any unused features are dead-code-eliminated in release builds _without_ having to rely on compile-time features flags!

For example, if your target doesn't implement a custom GDB `monitor` command handler, the resulting binary won't include any code related to parsing / handling the underlying `qRcmd` packet!

If you're interested in the low-level technical details of how IDETs work, I've included a brief writeup in the documentation [here](https://docs.rs/gdbstub/*/gdbstub/target/index.html#inlineable-dyn-extension-traits-idets).

## Feature flags

By default, the `std` and `derive_debug` features are enabled.

When using `gdbstub` in `#![no_std]` contexts, make sure to set `default-features = false`.

-   `alloc`
    -   Implements `Connection` for `Box<dyn Connection>`.
    -   Adds output buffering to `ConsoleOutput`.
    -   Log outgoing packets via `log::trace!` (uses a heap-allocated output buffer)
-   `std` (implies `alloc`)
    -   Implements `Connection` for [`TcpStream`](https://doc.rust-lang.org/std/net/struct.TcpStream.html) and [`UnixStream`](https://doc.rust-lang.org/std/os/unix/net/struct.UnixStream.html).
    -   Implements [`std::error::Error`](https://doc.rust-lang.org/std/error/trait.Error.html) for `gdbstub::Error`

## Examples

### `armv4t`

The `armv4t` example shows how `gdbstub` can be used to add `gdb` debugging support to an (incredibly simple) ARMv4T-based emulator. See `examples/armv4t/README.md` for details.

### `armv4t_multicore`

A dual-core variation of the `armv4t` example. Implements `gdbstub`'s multithread extensions to enable per-core debugging. See `examples/armv4t_multicore/README.md` for details.

## Real-World Examples

Several projects are already using `gdbstub`.

-   [clicky](https://github.com/daniel5151/clicky/) - An emulator for classic clickwheel iPods (dual-core ARMv4T SoC)
-   [rustyboyadvance-ng](https://github.com/michelhe/rustboyadvance-ng/) - Nintendo GameBoy Advance emulator and debugger
-   [microcorruption-emu](https://github.com/sapir/microcorruption-emu) - msp430 emulator for the microcorruption.com ctf
-   [ts7200](https://github.com/daniel5151/ts7200/) - An emulator for the TS-7200, a somewhat bespoke embedded ARMv4t platform

If you end up using `gdbstub` in your project, feel free to open a PR and add it to this list!

## Using `gdbstub` on bare-metal hardware

Since `gdbstub` is `#![no_std]` compatible, it should be possible to implement a `gdbstub::Target` which uses low-level trap instructions + context switching to debug bare-metal code.

If you happen to stumble across this crate and end up using it to debug some bare-metal code, please let me know! I'd love to link to your project, and/or create a simplified example based off your code!

## `unsafe` in `gdbstub`

`gdbstub` "core" only has 2 lines of unsafe code:

-   A call to `NonZeroUsize::new_unchecked(1)` when defining the `SINGLE_THREAD_TID` constant.
-   A call to `str::from_utf8_unchecked()` when working with incoming GDB packets (the underlying `&[u8]` buffer is checked with `is_ascii()` prior to the call).

With the `std` feature enabled, there is one additional line of `unsafe` code:

-   `gdbstub` includes an implementation of `UnixStream::peek` which uses `libc::recv`. This will be removed once [rust-lang/rust#73761](https://github.com/rust-lang/rust/pull/73761) is merged.

## Future Plans

-   Improve multiprocess / multi-thread / multi-core support
    -   Support thread-specific breakpoints
    -   Support non-stop mode?
    -   Support disabling multiprocess extensions on older GDB clients
-   Support addresses larger than 64-bits?
    -   This would require plumbing-through the architecture's pointer size as a generic parameter into all the packet parsing code, which probably isn't _too_ difficult, just time consuming.
