# gdbstub

[![](http://meritbadge.herokuapp.com/gdbstub)](https://crates.io/crates/gdbstub)
[![](https://docs.rs/gdbstub/badge.svg)](https://docs.rs/gdbstub)

An ergonomic and easy-to-integrate implementation of the [GDB Remote Serial Protocol](https://sourceware.org/gdb/onlinedocs/gdb/Remote-Protocol.html#Remote-Protocol) in Rust, with full `#![no_std]` support.

> _Note:_ [`gdbstub:master`](https://github.com/daniel5151/gdbstub/tree/master) only contains code that is semver-compatible with the latest released version of `gdbstub` (currently `0.4.x`). All breaking changes and \*major\* new features are staged in [`gdbstub:dev/0.5`](https://github.com/daniel5151/gdbstub/blob/dev/0.5/CHANGELOG.md#050-dev).
>
> I am expecting to cut a `0.5` release quite soon (hopefully before the end of May 2021), so if you're starting a `gdbstub` integration from scratch, I would highly recommend working off the `dev/0.5` branch directly, thereby saving yourself the effort of having to update from `0.4` to `0.5`.

Why use `gdbstub`?

-   **Excellent Ergonomics**
    -   Unlike other GDB stub libraries, which simply expose the underlying GDB protocol "warts and all", `gdbstub` tries to abstract as much of the raw GDB protocol details from the user.
        -   For example, instead of having to dig through some [obscure XML files deep the GDB codebase](https://github.com/bminor/binutils-gdb/tree/master/gdb/features) just to read/write from CPU registers, `gdbstub` comes with [built-in register definitions](https://docs.rs/gdbstub/*/gdbstub/arch/index.html) for most common architectures!
    -   `gdbstub` makes _extensive_ use of Rust's powerful type system + generics to enforce protocol invariants at compile time, minimizing the number of tricky protocol details end users have to worry about.
-   **Easy to Integrate**
    -   `gdbstub`'s API is designed to be as unobtrusive as possible, and shouldn't require any large refactoring effort to integrate into an existing project. It doesn't require taking direct ownership of any key data structures, and aims to be a "drop in" solution when you need to add debugging to a project.
-   **`#![no_std]` Ready & Size Optimized**
    -   Can be configured to use fixed-size, pre-allocated buffers. **`gdbstub` does _not_ depend on `alloc`.**
    -   `gdbstub` is transport-layer agnostic, and uses a basic [`Connection`](https://docs.rs/gdbstub/latest/gdbstub/trait.Connection.html) interface to communicate with the GDB server. As long as target has some method of performing in-order, serial, byte-wise I/O (e.g: putchar/getchar over UART), it's possible to run `gdbstub` on it.
    -   "You don't pay for what you don't use": If you don't implement a particular protocol extension, the resulting binary won't include _any_ code related to parsing/handling that extension's packets! See the [Zero-overhead Protocol Extensions](#zero-overhead-protocol-extensions) section below for more details.
    -   A lot of work has gone into reducing `gdbstub`'s binary and RAM footprints.
        -   In release builds, using all the tricks outlined in [`min-sized-rust`](https://github.com/johnthagen/min-sized-rust), a baseline `gdbstub` implementation weighs in at roughly **_10kb of `.text` and negligible `.rodata`!_** \*
        -   This is already pretty good, and I suspect that there are still lots of low-hanging optimizations which can reduce the size even further.

\* Exact numbers vary by target platform, compiler version, and `gdbstub` revision. Data was collected using the included `example_no_std` project compiled on x86_64.

`gdbstub` is particularly well suited for _emulation_, making it easy to add powerful, non-intrusive debugging support to an emulated system. Just provide an implementation of the [`Target`](https://docs.rs/gdbstub/latest/gdbstub/target/trait.Target.html) trait for your target platform, and you're ready to start debugging!

-   [Documentation](https://docs.rs/gdbstub)

### Can I Use `gdsbtub` in Production?

**Yes, as long as you don't mind some API churn until `1.0.0` is released.**

`gdbstub` has been integrated into [many projects](#real-world-examples) since its initial `0.1.0` release, and thusfar, no _major_ bugs have been reported. Reported issues have typically been the result of faulty `Target` implementations (e.g: forgetting to adjust the PC after a breakpoint is hit), or were related to certain unimplemented GDB protocol features.

That being said, due to `gdbstub`'s heavy use of Rust's type system in enforcing GDB protocol invariants at compile time, it's often been the case that implementing new GDB protocol features has required making some breaking Trait/Type changes (e.g: adding the `RegId` associated type to `Arch` to support addressing individual registers). While these changes are typically quite minor, they are nonetheless breaking, and may require a code-change when moving between versions.

See the [Future Plans + Roadmap to `1.0.0`](#future-plans--roadmap-to-100) for more information on what features `gdbstub` still needs to implement before committing to API stability with version `1.0.0`.

## Debugging Features

The GDB Remote Serial Protocol is surprisingly complex, supporting advanced features such as remote file I/O, spawning new processes, "rewinding" program execution, and much, _much_ more. Thankfully, most of these features are completely optional, and getting a basic debugging session up-and-running only requires implementing a few basic methods:

-   Base GDB Protocol
    -   Step + Continue
    -   Read/Write memory
    -   Read/Write registers
    -   (optional) Multithreading support

Of course, most use-cases will want to support additional debugging features as well. At the moment, `gdbstub` implements the following GDB protocol extensions:

-   Automatic architecture + feature detection (automatically implemented)
-   Breakpoints
    -   Software Breakpoints
    -   Hardware Breakpoints
    -   Read/Write/Access Watchpoints (i.e: value breakpoints)
-   Extended Mode
    -   Run/Attach/Kill Processes
    -   Pass environment variables / args to spawned processes
    -   Change working directory
-   Section offsets
    -   Get section/segment relocation offsets from the target
-   Custom `monitor` Commands
    -   Extend the GDB protocol with custom debug commands using GDB's `monitor` command

_Note:_ Which GDB features are implemented are decided on an as-needed basis by `gdbstub`'s contributors. If there's a missing GDB feature that you'd like `gdbstub` to implement, please file an issue / open a PR! Check out the [GDB Remote Configuration Docs](https://sourceware.org/gdb/onlinedocs/gdb/Remote-Configuration.html) for a table of GDB commands + their corresponding Remote Serial Protocol packets.

### Zero-overhead Protocol Extensions

Using a technique called **Inlineable Dyn Extension Traits** (IDETs), `gdbstub` is able to leverage the Rust compiler's powerful optimization passes to ensure any unused features are dead-code-eliminated in release builds _without_ having to rely on compile-time features flags!

For example, if your target doesn't implement a custom GDB `monitor` command handler, the resulting binary won't include any code related to parsing / handling the underlying `qRcmd` packet!

If you're interested in the low-level technical details of how IDETs work, I've included a brief writeup in the documentation [here](https://docs.rs/gdbstub/latest/gdbstub/target/ext/index.html#how-protocol-extensions-work---inlineable-dyn-extension-traits-idets).

## Feature flags

By default, the `std` and `alloc` features are enabled.

When using `gdbstub` in `#![no_std]` contexts, make sure to set `default-features = false`.

-   `alloc`
    -   Implement `Connection` for `Box<dyn Connection>`.
    -   Log outgoing packets via `log::trace!` (uses a heap-allocated output buffer).
    -   Provide built-in implementations for certain protocol features:
        -   Use a heap-allocated packet buffer in `GdbStub` (if none is provided via `GdbStubBuilder::with_packet_buffer`).
        -   (Monitor Command) Use a heap-allocated output buffer in `ConsoleOutput`.
-   `std` (implies `alloc`)
    -   Implement `Connection` for [`TcpStream`](https://doc.rust-lang.org/std/net/struct.TcpStream.html) and [`UnixStream`](https://doc.rust-lang.org/std/os/unix/net/struct.UnixStream.html).
    -   Implement [`std::error::Error`](https://doc.rust-lang.org/std/error/trait.Error.html) for `gdbstub::Error`.
    -   Add a `TargetError::Io` variant to simplify `std::io::Error` handling from Target methods.

## Examples

### Real-World Examples

-   Virtual Machine Monitors (VMMs)
    -   [crosvm](https://chromium.googlesource.com/chromiumos/platform/crosvm/+/refs/heads/main#gdb-support) - The Chrome OS Virtual Machine Monitor
    -   [Firecracker](https://firecracker-microvm.github.io/) - A lightweight VMM developed by AWS - feature is in [PR](https://github.com/firecracker-microvm/firecracker/pull/2168)
-   Emulators
    -   [clicky](https://github.com/daniel5151/clicky/) - An emulator for classic clickwheel iPods (dual-core ARMv4T SoC)
    -   [rustyboyadvance-ng](https://github.com/michelhe/rustboyadvance-ng/) - Nintendo GameBoy Advance emulator and debugger
    -   [ts7200](https://github.com/daniel5151/ts7200/) - An emulator for the TS-7200, a somewhat bespoke embedded ARMv4t platform
    -   [microcorruption-emu](https://github.com/sapir/microcorruption-emu) - msp430 emulator for the microcorruption.com ctf
-   Other
    -   [memflow](https://github.com/memflow/memflow) - A physical memory introspection framework (part of `memflow-cli`)

While some of these projects may use older versions of `gdbstub`, they can nonetheless serve as useful examples of what a typical `gdbstub` integration might look like.

If you end up using `gdbstub` in your project, consider opening a PR and add it to this list!

### In-tree "Toy" Examples

These examples are built as part of the CI, and are guaranteed to be kept up to date with the latest version of `gdbstub`'s API.

- `armv4t` - `./examples/armv4t/`
    - An incredibly simple ARMv4T-based system emulator with `gdbstub` support.
    - Unlike all other examples, `armv4t` **implements (almost) all available `target::ext` features.**
- `armv4t_multicore` - `./examples/armv4t_multicore/`
    - A dual-core variation of the `armv4t` example.
    - Implements the core of `gdbstub`'s multithread extensions API, but not much else.
- `example_no_std` - `./example_no_std`
    - An _extremely_ minimal example of how `gdbstub` can be used in a `#![no_std]` project.
    - Unlike the `armv4t/armv4t_multicore` examples, this project does _not_ include a working emulator, and stubs-out all `gdbstub` functions.
    - Tracks `gdbstub`'s approximate binary footprint (via the `check_size.sh` script)

## Using `gdbstub` on bare-metal hardware

Quite a bit of work has gone into making `gdbstub` optimized for `#![no_std]`, which means it should be entirely possible to implement a `Target` which uses low-level trap instructions + context switching to debug bare-metal code.

If you happen to stumble across this crate and end up using it to debug some bare-metal code, please let me know! I'd love to link to your project, and/or create a simplified example based off your code!

## `unsafe` in `gdbstub`

`gdbstub` limits the use of `unsafe` to a bare minimum. All uses of `unsafe` are required to have a corresponding `// SAFETY` comment, and are exhaustively documented in the list below.

-   When no features are enabled:
    -   A few trivially safe calls to `NonZeroUsize::new_unchecked()` when defining internal constants.
-   When the `std` feature is enabled:
    -   An implementation of `UnixStream::peek` which uses `libc::recv`. This will be removed once [rust-lang/rust#73761](https://github.com/rust-lang/rust/pull/73761) is merged and stabilized.

## Future Plans + Roadmap to `1.0.0`

Before `gdbstub` can comfortably commit to a stable `1.0.0` API, there are several outstanding features that should be implemented and questions that need to be addressed. Due to `gdbstub`'s heavy reliance on the Rust type system to enforce GDB protocol invariants, it's likely that a certain subset of yet-unimplemented protocol features may require breaking API changes.

Notably, the vast majority of GDB protocol features (e.g: remote filesystem support, tracepoint packets, most query packets, etc...) should _not_ require breaking API changes, and could most likely be implemented using the standard backwards-compatible protocol extension approach.

The following features are most likely to require breaking API changes, and should therefore be implemented prior to `1.0.0`.

-   [ ] Stabilize the `Arch` trait
    -   [ ] Allow fine-grained control over target features ([\#12](https://github.com/daniel5151/gdbstub/issues/12))
    -   [ ] Remove `RawRegId` ([\#29](https://github.com/daniel5151/gdbstub/issues/29))
-   [ ] Implement GDB's various high-level operating modes:
    -   [x] Single/Multi Thread debugging
    -   [ ] Multiprocess Debugging
        -   [ ] Add a third `base::multiprocess` API.
        -   _Note:_ `gdbstub` already implements multiprocess extensions "under-the-hood", and just hard-codes a fake PID.
    -   [x] [Extended Mode](https://sourceware.org/gdb/current/onlinedocs/gdb/Connecting.html) (`target extended-remote`)
    -   [ ] [Non-Stop Mode](https://sourceware.org/gdb/onlinedocs/gdb/Remote-Non_002dStop.html#Remote-Non_002dStop)
        -   This may require some breaking API changes and/or some internals rework -- more research is needed.
-   [ ] Have a working example of `gdbstub` running in a "bare-metal" `#![no_std]` environment (e.g: debugging a hobby OS via serial).
    -   While there's no reason it _wouldn't_ work, it would be good to validate that the API + implementation supports this use-case.

Additionally, while not strict "blockers" to `1.0.0`, it would be good to explore these features as well:

-   [ ] Commit to a MSRV
-   [ ] Exposing an `async/await` interface
    -   e.g: the current `check_gdb_interrupt` callback in `Target::resume()` could be modeled as a future.
    -   Would require some tweaks to the Connection trait.
-   [ ] Adding [LLDB extension](https://raw.githubusercontent.com/llvm-mirror/lldb/master/docs/lldb-gdb-remote.txt) support
    -   Skimming through the list, it doesn't seem like these extensions would require breaking API changes -- more research is needed.
