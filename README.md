# gdbstub

[![](http://meritbadge.herokuapp.com/gdbstub)](https://crates.io/crates/gdbstub)
[![](https://docs.rs/gdbstub/badge.svg)](https://docs.rs/gdbstub)

An ergonomic and easy-to-integrate implementation of the [GDB Remote Serial Protocol](https://sourceware.org/gdb/onlinedocs/gdb/Remote-Protocol.html#Remote-Protocol) in Rust, with full `#![no_std]` support.

`gdbstub`  makes it easy to integrate powerful guest debugging support to your emulator/hypervisor/debugger/embedded project. By implementing just a few basic methods of the [`gdbstub::Target`](https://docs.rs/gdbstub/latest/gdbstub/target/ext/base/singlethread/trait.SingleThreadOps.html) trait, you can have a rich GDB debugging session up and running in no time!

**If you're looking for a quick snippet of example code to see what a typical `gdbstub` integration might look like, check out [examples/armv4t/gdb/mod.rs](https://github.com/daniel5151/gdbstub/blob/master/examples/armv4t/gdb/mod.rs)**

-   [Documentation (gdbstub)](https://docs.rs/gdbstub)
-   [Documentation (gdbstub_arch)](https://docs.rs/gdbstub_arch)
-   [Changelog](CHANGELOG.md)
-   [0.4 to 0.5 Transition Guide](docs/transition_guide.md)

Why use `gdbstub`?

-   **Excellent Ergonomics**
    -   Instead of simply exposing the underlying GDB protocol "warts and all", `gdbstub` tries to abstract as much of the raw GDB protocol details from the user. For example:
        -   Instead of having to dig through [obscure XML files deep the GDB codebase](https://github.com/bminor/binutils-gdb/tree/master/gdb/features) just to read/write from CPU/architecture registers, `gdbstub` comes with a community-curated collection of [built-in architecture definitions](https://docs.rs/gdbstub_arch) for most popular platforms!
        -   Organizes GDB's countless optional protocol extensions into a coherent, understandable, and type-safe hierarchy of traits.
        -   Automatically handles client/server protocol feature negotiation, without needing to micro-manage the specific [`qSupported` packet](https://sourceware.org/gdb/onlinedocs/gdb/General-Query-Packets.html#qSupported) response.
    -   `gdbstub` makes _extensive_ use of Rust's powerful type system + generics to enforce protocol invariants at compile time, minimizing the number of tricky protocol details end users have to worry about.
    -   Using a novel technique called [**Inlineable Dyn Extension Traits**](#zero-overhead-protocol-extensions) (IDETs), `gdbstub` enables fine-grained control over active protocol extensions _without_ relying on clunky `cargo` features or the use of `unsafe` code!
-   **Easy to Integrate**
    -   `gdbstub`'s API is designed to be as unobtrusive as possible, and shouldn't require any large refactoring effort to integrate into an existing project. It doesn't require taking direct ownership of any key data structures, and aims to be a "drop in" solution when you need to add debugging to a project.
-   **`#![no_std]` Ready & Size Optimized**
    -   `gdbstub` is a **`no_std` first** library, whereby all protocol features are required to be `no_std` compatible.
    -   `gdbstub` does not require _any_ dynamic memory allocation, and can be configured to use fixed-size, pre-allocated buffers. This enables `gdbstub` to be used on even the most resource constrained, no-[`alloc`](https://doc.rust-lang.org/alloc/) platforms.
    -   `gdbstub` is entirely **panic free** (when compiled in release mode, without the `paranoid_unsafe` cargo feature).
        -   Validated by inspecting the asm output of the in-tree `example_no_std`.
    -   `gdbstub` is transport-layer agnostic, and uses a basic [`Connection`](https://docs.rs/gdbstub/latest/gdbstub/trait.Connection.html) interface to communicate with the GDB server. As long as target has some method of performing in-order, serial, byte-wise I/O (e.g: putchar/getchar over UART), it's possible to run `gdbstub` on it!
    -   "You don't pay for what you don't use": All code related to parsing/handling protocol extensions is guaranteed to be dead-code-eliminated from an optimized binary if left unimplemented! See the [Zero-overhead Protocol Extensions](#zero-overhead-protocol-extensions) section below for more details.
    -   `gdbstub`'s minimal configuration has an incredibly low binary size + RAM overhead, enabling it to be used on even the most resource-constrained microcontrollers.
        -   When compiled in release mode, using all the tricks outlined in [`min-sized-rust`](https://github.com/johnthagen/min-sized-rust), a baseline `gdbstub` implementation can weigh in at **_less than 10kb of `.text` + `.rodata`!_** \*
        - \*Exact numbers vary by target platform, compiler version, and `gdbstub` revision. Data was collected using the included `example_no_std` project compiled on x86_64.

### Can I Use `gdbstub` in Production?

**Yes, as long as you don't mind some API churn until `1.0.0` is released.**

Due to `gdbstub`'s heavy use of Rust's type system in enforcing GDB protocol invariants at compile time, it's often been the case that implementing new GDB protocol features has required making some breaking Trait/Type changes. While these changes are typically quite minor, they are nonetheless semver-breaking, and may require a code-change when moving between versions. Any particularly involved changes will typically be documented in a dedicated [transition guide](docs/transition_guide.md) document.

That being said, `gdbstub` has already been integrated into [many real-world projects](#real-world-examples) since its initial `0.1` release, and empirical evidence suggests that it seems to be doing its job quite well! Thusfar, there haven't been any reported issues related to core GDB debugging functionality, with most issues being caused by faulty `Target` and/or `Arch` implementations.

See the [Future Plans + Roadmap to `1.0.0`](#future-plans--roadmap-to-100) for more information on what features `gdbstub` still needs to implement before committing to API stability with version `1.0.0`.

## Debugging Features

The GDB Remote Serial Protocol is surprisingly complex, supporting advanced features such as remote file I/O, spawning new processes, "rewinding" program execution, and much, _much_ more. Thankfully, most of these features are completely optional, and getting a basic debugging session up-and-running only requires implementing a few basic methods:

-   Base GDB Protocol
    -   Step + Continue
    -   Read/Write memory
    -   Read/Write registers
    -   Enumerating threads
        -   Only required in multithreaded targets

Of course, most use-cases will want to support additional debugging features as well. At the moment, `gdbstub` implements the following GDB protocol extensions:

-   Automatic target architecture + feature reporting
-   Breakpoints
    -   Software Breakpoints
    -   Hardware Breakpoints
    -   Read/Write/Access Watchpoints (i.e: value breakpoints)
-   Advanced step/continue
    -   Reverse execution (reverse-step, reverse-continue)
    -   Range-stepping
-   Extended Mode
    -   Run/Attach/Kill Processes
    -   Pass environment variables / args to spawned processes
    -   Change working directory
-   Section offsets
    -   Get section/segment relocation offsets from the target
-   Custom `monitor` Commands
    -   Extend the GDB protocol with custom debug commands using GDB's `monitor` command
-   Get target memory map
-   Perform Host I/O operations
-   Get target exec file

_Note:_ GDB features are implemented on an as-needed basis by `gdbstub`'s contributors. If there's a missing GDB feature that you'd like `gdbstub` to implement, please file an issue and/or open a PR!

For a full list of GDB remote features, check out the [GDB Remote Configuration Docs](https://sourceware.org/gdb/onlinedocs/gdb/Remote-Configuration.html) for a table of GDB commands + their corresponding Remote Serial Protocol packets.

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
-   `paranoid_unsafe`
    -   Enabling the `paranoid_unsafe` feature will swap out a handful of unsafe `get_unchecked_mut` operations with their safe equivalents, at the expense of introducing panicking code into `gdbstub`.
        -   `rustc` + LLVM do a pretty incredible job at eliding bounds checks... most of the time. Unfortunately, there are a few places in the code where the compiler is not smart enough to "prove" that a bounds check isn't needed, and a bit of unsafe code is required to remove those bounds checks.
    -   This feature is **disabled** by default, as the unsafe code has been aggressively audited and tested for correctness. That said, if you're particularly paranoid about the use of unsafe code, enabling this feature may offer some piece of mind.
    -   Please refer to the [`unsafe` in `gdbstub`](#unsafe-in-gdbstub) section below for more details.

## Examples

### Real-World Examples

-   Virtual Machine Monitors (VMMs)
    -   [crosvm](https://chromium.googlesource.com/chromiumos/platform/crosvm/+/refs/heads/main#gdb-support) - The Chrome OS Virtual Machine Monitor (x64)
    -   [Firecracker](https://firecracker-microvm.github.io/) - A lightweight VMM developed by AWS - feature is in [PR](https://github.com/firecracker-microvm/firecracker/pull/2168)
-   Emulators (x64)
    -   [clicky](https://github.com/daniel5151/clicky/) - An emulator for classic clickwheel iPods (dual-core ARMv4T SoC)
    -   [rustyboyadvance-ng](https://github.com/michelhe/rustboyadvance-ng/) - Nintendo GameBoy Advance emulator and debugger (ARMv4T)
    -   [vaporstation](https://github.com/Colin-Suckow/vaporstation) - A Playstation One emulator (MIPS)
    -   [ts7200](https://github.com/daniel5151/ts7200/) - An emulator for the TS-7200, a somewhat bespoke embedded ARMv4t platform
    -   [microcorruption-emu](https://github.com/sapir/microcorruption-emu) - msp430 emulator for the microcorruption.com ctf
-   Other
    -   [memflow](https://github.com/memflow/memflow-cli) - A physical memory introspection framework (part of `memflow-cli`) (64)

While some of these projects may use older versions of `gdbstub`, they can nonetheless serve as useful examples of what a typical `gdbstub` integration might look like.

If you end up using `gdbstub` in your project, consider opening a PR and adding it to this list!

### In-tree "Toy" Examples

These examples are built as part of the CI, and are guaranteed to be kept up to date with the latest version of `gdbstub`'s API.

- `armv4t` - `./examples/armv4t/`
    - An incredibly simple ARMv4T-based system emulator with `gdbstub` support.
    - **Implements (almost) all available `target::ext` features.** This makes it a great resource when first implementing a new protocol extension!
- `armv4t_multicore` - `./examples/armv4t_multicore/`
    - A dual-core variation of the `armv4t` example.
    - Implements the core of `gdbstub`'s multithread extensions API, but not much else.
- `example_no_std` - `./example_no_std`
    - An _extremely_ minimal example which shows off how `gdbstub` can be used in a `#![no_std]` project.
    - Unlike the `armv4t/armv4t_multicore` examples, this project does _not_ include a working emulator, and simply stubs all `gdbstub` functions.
    - Doubles as a test-bed for tracking `gdbstub`'s approximate binary footprint (via the `check_size.sh` script), and validating certain dead-code-elimination optimizations.

## Using `gdbstub` on bare-metal hardware

Quite a bit of work has gone into making `gdbstub` optimized for `#![no_std]`, which means it should be entirely possible to implement a `Target` which uses low-level trap instructions + context switching to debug bare-metal code.

If you happen to stumble across this crate and end up using it to debug some bare-metal code, please let me know! I'd love to link to your project, and/or create a simplified example based off your code!

## `unsafe` in `gdbstub`

`gdbstub` limits its use of `unsafe` to a bare minimum, with all uses of `unsafe` required to have a corresponding `// SAFETY` comment as justification. The following list exhaustively documents all uses of `unsafe` in `gdbstub`.

-   When no cargo features are enabled:
    -   A few trivially safe calls to `NonZeroUsize::new_unchecked()` when defining internal constants.

-   When the `paranoid_unsafe` feature is enabled, the following `unsafe` code is _removed_:
    -   `src/protocol/packet.rs`: Swaps a couple slice-index methods in `PacketBuf` to use `get_unchecked_mut`. The public API of struct ensures that the bounds used to index into the array remain in-bounds.
    -   `src/protocol/common/hex`: Use an alternate implementation of `decode_hex_buf`/`decode_bin_buf` which uses unsafe slice indexing.

-   When the `std` feature is enabled:
    -   `src/connection/impls/unixstream.rs`: An implementation of `UnixStream::peek` which uses `libc::recv`. This manual implementation will be removed once [rust-lang/rust#76923](https://github.com/rust-lang/rust/issues/76923) is stabilized.

## Future Plans + Roadmap to `1.0.0`

While the vast majority of GDB protocol features (e.g: remote filesystem support, tracepoint packets, most query packets, etc...) should _not_ require breaking API changes, there are still several key protocol features that'll need breaking API changes to be implemented.

The following features are most likely to require breaking API changes, and should therefore be implemented prior to `1.0.0`. Not that this is _not_ an exhaustive list, and is subject to change.

-   [ ] Stabilize the `Arch` trait
    -   [ ] Allow fine-grained control over target features ([\#12](https://github.com/daniel5151/gdbstub/issues/12))
    -   [ ] Remove `RawRegId` ([\#29](https://github.com/daniel5151/gdbstub/issues/29))
-   [ ] Implement GDB's various high-level operating modes:
    -   [x] Single/Multi Thread debugging
    -   [ ] Multiprocess Debugging
        -   [ ] Will require adding a third `target::ext::base::multiprocess` API.
        -   _Note:_ `gdbstub` already implements multiprocess extensions "under-the-hood", and just hard-codes a fake PID, so this is mostly a matter of "putting in the work".
    -   [x] [Extended Mode](https://sourceware.org/gdb/current/onlinedocs/gdb/Connecting.html) (`target extended-remote`)
    -   [ ] [Non-Stop Mode](https://sourceware.org/gdb/onlinedocs/gdb/Remote-Non_002dStop.html#Remote-Non_002dStop)
        -   This may require some breaking API changes and/or some internals rework -- more research is needed.
-   [ ] Have a working example of `gdbstub` running in a "bare-metal" `#![no_std]` environment (e.g: debugging a hobby OS via serial).
    -   While there's no reason it _shouldn't_ work, it would be good to validate that the API + implementation supports this use-case.

Additionally, while not strict "blockers" to `1.0.0`, it would be good to explore these features as well:

-   [ ] Should `gdbstub` commit to a MSRV?
-   [ ] Exposing `async/await` interfaces (particularly wrt. handling GDB client interrupts) ([\#36](https://github.com/daniel5151/gdbstub/issues/36))
-   [ ] Supporting various [LLDB extensions](https://raw.githubusercontent.com/llvm-mirror/lldb/master/docs/lldb-gdb-remote.txt) to the GDB RSP
    -   Skimming through the list, it doesn't seem like these extensions would require breaking API changes -- more research is needed.
-   [ ] Supporting multi-arch debugging via a single target
    -   e.g: debugging both x86 and x64 processes when running in extended mode
-   [ ] Proper handling of client "nack" packets for spotty connections.
