All notable changes to this project will be documented in this file.

This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

# 0.7.3

#### New Features

- Add new `core_error` feature, to have `GdbStubError` impl `core::error::Error`. [\#154](https://github.com/daniel5151/gdbstub/pull/154) ([ultimaweapon](https://github.com/ultimaweapon))
  - _Note:_ Out of an abundance of caution, this has been put behind a
    feature-flag, as while `gdbstub` doesn't claim a strict MSRV at this time,
    it seemed unwise to have a PATCH release break folks stuck on a pre-1.81
    Rust toolchain.

# 0.7.2

#### Bugfixes

- Add workaround for vCont packets that specify a '0' (Any) thread-id
  - For more context, see [`e9a5296c`](https://github.com/daniel5151/gdbstub/commit/e9a5296c4d02f4b5b73d5738654a33d01afa8711)

#### Internal Improvements

- Various README tweaks
- Various clippy lint fixes
- Fix incorrect valid-addr check in armv4t example

# 0.7.1

#### New Protocol Extensions

- `LibrariesSvr4` - List an SVR4 (System-V/Unix) target's libraries. [\#142](https://github.com/daniel5151/gdbstub/pull/142) ([alexcrichton](https://github.com/alexcrichton))

# 0.7.0

#### Breaking API Changes

- `stub::GdbStubError` is now an opaque `struct` with a handful of methods to extract user-defined context (as opposed to being an `enum` that directly exposed all error internals to the user).
  - _This change will enable future versions of `gdbstub` to fearlessly improve error messages and infrastructure without making semver breaking changes. See [\#112](https://github.com/daniel5151/gdbstub/pull/132) for more._
- `common::Signal` is not longer an `enum`, and is instead a `struct` with a single `pub u8` field + a collection of associated constants.
  - _As a result, yet another instance of `unsafe` could be removed from the codebase!_
- `Arch` API:
  - Entirely removed `single_step_behavior`. See [\#132](https://github.com/daniel5151/gdbstub/pull/132) for details and rationale
- `Target` APIs:
  - `SingleThreadBase`/`MultiThreadBase`
    - `read_addrs` now returns a `usize` instead of a `()`, allowing implementations to report cases where only a subset of memory could be read. [\#115](https://github.com/daniel5151/gdbstub/pull/115) ([geigerzaehler](https://github.com/geigerzaehler))
  - `HostIo`
    - `bitflags` has been updated from `1.x` to `2.x`, affecting the type of `HostIoOpenFlags` and `HostIoOpenMode` [\#138](https://github.com/daniel5151/gdbstub/pull/138) ([qwandor](https://github.com/qwandor))

#### Internal Improvements

- Reformatted codebase with nightly rustfmt using `imports_granularity = "Item"`

# 0.6.6

#### New Features

- `Target::use_no_ack_mode` - toggle support for for activating "no ack mode" [\#135](https://github.com/daniel5151/gdbstub/pull/135) ([bet4it](https://github.com/bet4it))

# 0.6.5

#### New Protocol Extensions

- `ExtendedMode > CurrentActivePid` - Support reporting a non-default active PID [\#133](https://github.com/daniel5151/gdbstub/pull/129)
  - Required to fix `vAttach` behavior (see Bugfixes section below)

#### Bugfixes

- Fix for targets with no active threads [\#127](https://github.com/daniel5151/gdbstub/pull/127) ([xobs](https://github.com/xobs))
- Fix `vAttach` behavior when switching between multiple processes [\#129](https://github.com/daniel5151/gdbstub/pull/129) ([xobs](https://github.com/xobs)), and [\#133](https://github.com/daniel5151/gdbstub/pull/129)
- Minor doc fixes

# 0.6.4

#### Bugfixes

- Avoid truncating `X` packets that contain `:` and `,` as part of the payload. [\#121](https://github.com/daniel5151/gdbstub/pull/121) ([709924470](https://github.com/709924470))

#### Internal Improvements

- Various README tweaks
- Remove some `unsafe` code
- CI improvements
  - Run no-panic checks on `example_no_std`
  - Run CI on docs

# 0.6.3

#### New Features

- `SingleRegisterAccess`: Support reporting unavailable regs [\#107](https://github.com/daniel5151/gdbstub/pull/107) ([ptosi](https://github.com/ptosi))

# 0.6.2

#### New Protocol Extensions

- `MultiThreadBase > ThreadExtraInfo` - Provide extra information per-thread. [\#106](https://github.com/daniel5151/gdbstub/pull/106) ([thefaxman](https://github.com/thefaxman))
- `LldbRegisterInfo` - (LLDB specific) Report register information in the LLDB format. [\#103](https://github.com/daniel5151/gdbstub/pull/103) ([jawilk](https://github.com/jawilk))
  - This information can be statically included as part of the `Arch` implemention, or dynamically reported via the `LldbRegisterInfoOverride` IDET.

#### Bugfixes

- Report thread ID in response to `?` packet. [\#105](https://github.com/daniel5151/gdbstub/pull/105) ([thefaxman](https://github.com/thefaxman))

#### Internal Improvements

- Tweak enabled clippy lints
- Added a light dusting of `#[inline]` across the packet parsing code, crunching the code down even further
- Expanded on "no-panic guarantee" docs

# 0.6.1

#### New Features

- add LLDB-specific HostIoOpenFlags [\#100](https://github.com/daniel5151/gdbstub/pull/100) ([mrk](https://github.com/mrk-its))

# 0.6.0

After over a half-year of development, `gdbstub` 0.6 has finally been released!

This massive release delivers a slew of new protocol extensions, internal improvements, and key API improvements. Some highlights include:

- A new _non-blocking_ `GdbStubStateMachine` API, enabling `gdbstub` to integrate nicely with async event loops!
  - Moreover, on `no_std` platforms, this new API enables `gdbstub` to be driven directly via breakpoint/serial interrupt handlers!
  - This API is already being used in several Rust kernel projects, such as [`vmware-labs/node-replicated-kernel`](https://github.com/vmware-labs/node-replicated-kernel/tree/4326704/kernel/src/arch/x86_64/gdb) and [`betrusted-io/xous-core`](https://github.com/betrusted-io/xous-core/blob/7d3d710/kernel/src/debug/gdb_server.rs) to enable bare-metal, in-kernel debugging.
- `gdbstub` is now entirely **panic free** in release builds!
  - \* subject to `rustc`'s compiler optimizations
  - This was a pretty painstaking effort, but the end result is a substantial reduction in binary size on `no_std` platforms.
- Tons of new and exciting protocol extensions, including but not limited to:
  - Support for remote file I/O (reading/writing files to the debug target)
  - Fetching remote memory maps
  - Catching + reporting syscall entry/exit conditions
  - ...and many more!
- A new license: `gdbstub` is licensed under MIT OR Apache-2.0

See the [changelog](https://github.com/daniel5151/gdbstub/blob/dev/0.6/CHANGELOG.md) for a comprehensive rundown of all the new features.

While this release does come with quite a few breaking changes, the core IDET-based `Target` API has remained much the same, which should make porting code over from 0.5.x to 0.6 pretty mechanical. See the [`transition_guide.md`](./docs/transition_guide.md) for guidance on upgrading from `0.5.x` to `0.6`.

And as always, a huge shoutout to the folks who contributed PRs, Issues, and ideas to `gdbstub` - this release wouldn't have been possible without you! Special shoutouts to [gz](https://github.com/gz) and [xobs](https://github.com/xobs) for helping me test and iterate on the new bare-metal state machine API, and [bet4it](https://github.com/bet4it) for pointing out and implementing many useful API improvements and internal refactors.

Cheers!

#### New Features

- The new `GdbStubStateMachine` API gives users the power and flexibility to integrate `gdbstub` into their project-specific event loop infrastructure.
  - e.g: A global instance of `GdbStubStateMachine` can be driven directly from bare-metal interrupt handlers in `no_std` environments
  - e.g: A project using `async`/`await` can wrap `GdbStubStateMachine` in a task, yielding execution while waiting for the target to resume / new data to arrive down the `Connection`
- Removed all panicking code from `gdbstub`
  - See the [commit message](https://github.com/daniel5151/gdbstub/commit/ecbbaf72e01293b410ef3bc5970d18aa81e45599) for more details on how this was achieved.
- Introduced strongly-typed enum for protocol defined signal numbers (instead of using bare `u8`s)
- Added basic feature negotiation to support clients that don't support `multiprocess+` extensions.
- Relicensed `gdbstub` under MIT OR Apache-2.0 [\#68](https://github.com/daniel5151/gdbstub/pull/68)
- Added several new "guard rails" to avoid common integration footguns:
  - `Target::guard_rail_implicit_sw_breakpoints` - guards against the GDB client silently overriding target instructions with breakpoints if `SwBreakpoints` hasn't been implemented.
  - `Target::guard_rail_single_step_gdb_behavior` - guards against a GDB client bug where support for single step may be required / ignored on certain platforms (e.g: required on x86, ignored on MIPS)
- Added several new "toggle switches" to enable/disable parts of the protocol (all default to `true`)
  - `Target::use_x_upcase_packet` - toggle support for the more efficient `X` memory write packet
  - `Target::use_resume_stub` - toggle `gdbstub`'s built-in "stub" resume handler that returns `SIGRAP` if a target doesn't implement support for resumption
  - `Target::use_rle` - toggle whether outgoing packets are Run Length Encoded (RLE)

#### New Protocol Extensions

- `MemoryMap` - Get memory map XML file from the target. [\#54](https://github.com/daniel5151/gdbstub/pull/54) ([Tiwalun](https://github.com/Tiwalun))
- `CatchSyscalls` - Enable and disable catching syscalls from the inferior process. [\#57](https://github.com/daniel5151/gdbstub/pull/57) ([mchesser](https://github.com/mchesser))
- `HostIo` - Perform I/O operations on host. [\#66](https://github.com/daniel5151/gdbstub/pull/66) ([bet4it](https://github.com/bet4it))
  - Support for all Host I/O operations: `open`, `close`, `pread`, `pwrite`, `fstat`, `unlink`, `readlink`, `setfs`
- `ExecFile` - Get full absolute path of the file that was executed to create a process running on the remote system. [\#69](https://github.com/daniel5151/gdbstub/pull/69) ([bet4it](https://github.com/bet4it))
- `Auxv` - Access the target’s auxiliary vector. [\#86](https://github.com/daniel5151/gdbstub/pull/86) ([bet4it](https://github.com/bet4it))
- Implement `X` packet - More efficient bulk-write to memory (superceding the `M` packet). [\#82](https://github.com/daniel5151/gdbstub/pull/82) ([gz](https://github.com/gz))

#### Breaking API Changes

- `Connection` API:
  - Removed the `read` and `peek` methods from `Connection`
    - These have been moved to the new `ConnectionExt` trait, which is used in the new `GdbStub::run_blocking` API
- `Arch` API:
  - Dynamic read_register + RegId support. [\#85](https://github.com/daniel5151/gdbstub/pull/85) ([bet4it](https://github.com/bet4it))
- `Target` APIs:
  - prefix all IDET methods with `support_`
    - _makes it far easier to tell at-a-glance whether a method is an IDET, or an actual handler method.
  - Introduce strongly-typed enum for protocol defined signal numbers (instead of using bare `u8`s)
  - `Base` API:
    - Make single-stepping optional [\#92](https://github.com/daniel5151/gdbstub/pull/92)
    - Remove `GdbInterrupt` type (interrupt handling lifted to higher-level APIs)
    - Remove `ResumeAction` type (in favor of separate methods for various resume types)
  - `Breakpoints` API:
    - `HwWatchpoint`: Plumb watchpoint `length` parameter to public API
  - `TargetXml` API:
    - Support for `<xi:include>` in target.xml, which required including the `annex` parameter in the handler method.
    - `annex` is set to `b"target.xml"` on the fist call, though it may be set to other values in subsequent calls if `<xi:include>` is being used.
  - Pass `PacketBuf`-backed `&mut [u8]` as a response buffer to various APIs [\#72](https://github.com/daniel5151/gdbstub/pull/72) ([bet4it](https://github.com/bet4it))
    - Improvement over the callback-based approach.
    - This change is possible thanks to a clause in the GDB spec that specifies that responses will never exceed the size of the `PacketBuf`.
    - Also see [\#70](https://github.com/daniel5151/gdbstub/pull/70), which tracks some other methods that might be refactored to use this approach in the future.

#### Internal Improvements

- Documentation
  - Fix crates.io badges [\#71](https://github.com/daniel5151/gdbstub/pull/71) ([atouchet](https://github.com/atouchet))
  - Add `uhyve` to real-world examples [\#73](https://github.com/daniel5151/gdbstub/pull/73) ([mkroening](https://github.com/mkroening))
- Use stable `clippy` in CI
- Enable logging for responses with only alloc [\#78](https://github.com/daniel5151/gdbstub/pull/78) ([gz](https://github.com/gz))
- Lots of internal refactoring and cleanup

# 0.5.0

While the overall structure of the API has remained the same, `0.5.0` does introduce a few breaking API changes that require some attention. That being said, it should not be a difficult migration, and updating to `0.5.0` from `0.4` shouldn't take more than 10 mins of refactoring.

Check out [`transition_guide.md`](./docs/transition_guide.md) for guidance on upgrading from `0.4.x` to `0.5`.

#### New Features

- Implement Run-Length-Encoding (RLE) on outgoing packets
  - _This significantly cuts down on the data being transferred over the wire when reading from registers/memory_
- Add target-specific `kind: Arch::BreakpointKind` parameters to the Breakpoint API
  - _While emulated systems typically implement breakpoints by pausing execution once the PC hits a certain value, "real" systems typically need to patch the instruction stream with a breakpoint instruction. On systems with variable-sized instructions, this `kind` parameter specifies the size of the instruction that should be injected._
- Implement `ResumeAction::{Step,Continue}WithSignal`
- Added the `Exited(u8)`, `Terminated(u8)`, and `ReplayLog("begin"|"end")` stop reasons.
- Added `DisconnectReason::Exited(u8)` and `DisconnectReason::Terminated(u8)`.
- Reworked the `MultiThreadOps::resume` API to be significantly more ergonomic and efficient
  - See the [transition guide](https://github.com/daniel5151/gdbstub/blob/master/docs/transition_guide.md#new-multithreadopsresume-api) for more details.

#### New Protocol Extensions

- `{Single,Multi}ThreadReverse{Step,Continue}` - Support for reverse-step and reverse-continue. [\#48](https://github.com/daniel5151/gdbstub/pull/48 ) ([DrChat](https://github.com/DrChat))
- `{Single,Multi}ThreadRangeStepping` - Optional optimized [range stepping](https://sourceware.org/gdb/current/onlinedocs/gdb/Continuing-and-Stepping.html#range-stepping) support.

#### Breaking Arch Changes

- **`gdbstub::arch` has been moved into a separate `gdbstub_arch` crate**
  - _See [\#45](https://github.com/daniel5151/gdbstub/issues/45) for details on why this was done._
- (x86) Break GPRs & SRs into individual fields/variants [\#34](https://github.com/daniel5151/gdbstub/issues/34)

#### Breaking API Changes

- Base Protocol Refactors
  - Reworked the `MultiThreadOps::resume` API
  - Added a wrapper around the raw `check_gdb_interrupt` callback, hiding the underlying implementation details
  - Extracted base protocol single-register access methods (`{read,write}_register`) into separate `SingleRegisterAccess` trait
    - _These are optional GDB protocol methods, and as such, should be modeled as IDETs_
- Protocol Extension Refactors
  - Consolidated the `{Hw,Sw}Breakpoints/Watchpoints` IDETs under a single `Breakpoints` IDET + sub-IDETs
  - Added new arch-specific `kind: Arch::BreakpointKind` parameter to `add_{hw,sw}_breakpoint` methods
  - Renamed `target::ext::extended_mod::ConfigureASLR{Ops}` to `ConfigureAslr{Ops}` (clippy::upper_case_acronyms)
- Added `{Step,Continue}WithSignal` variants to `target::ext::base::ResumeAction`
- Trait Changes
  - `arch::Arch`: Added `type BreakpointKind`. Required to support arch-specific breakpoint kinds
  - `arch::Arch`: (very minor) Added [`num_traits::FromPrimitive`](https://docs.rs/num/0.4.0/num/traits/trait.FromPrimitive.html) bound to `Arch::Usize`
  - `arch::Registers`: Added `type ProgramCounter` and associated `fn pc(&self) -> Self::ProgramCounter` method. Added preemptively in anticipation of future GDB Agent support
- Removed the `Halted` stop reason (more accurate to simply return `{Exited|Terminated}(SIGSTOP)` instead).
- Removed the `Halted` disconnect reason (replaced with the `Exited` and `Terminated` stop reasons instead).
- Removed the implicit `ExtendedMode` attached PID tracking when `alloc` was available. See [`23b56038`](https://github.com/daniel5151/gdbstub/commit/23b56038) rationale behind this change.


#### Internal Improvements

- Split monolithic `GdbStubImpl` implementation into separate files (by protocol extension)
- Finally rewrite + optimize `GdbStubImpl::do_vcont`, along with streamlining its interactions with the legacy `s` and `c` packets
- Sprinkle more IDET-based dead code elimination hints (notably wrt. stop reasons)
- Remove the default `self.current_mem_tid` hack, replacing it with a much more elegant solution
- Packet Parser improvements
  - Remove last remaining bit of UTF-8 related code
  - Eliminate as much panicking bounds-checking code as possible
  - support efficient parsing of packets that are parsed differently depending on active protocol extension (namely, the breakpoint packets)
  - (currently unused) Zero-cost support for parsing `Z` and `z` packets with embedded agent bytecode expressions
- Use intra-doc links whenever possible

#### Bugfixes

- Fix `RiscvRegId` for `arch::riscv::Riscv64` [\#46](https://github.com/daniel5151/gdbstub/issues/46) ([fzyz999](https://github.com/fzyz999))

# 0.4.5

#### New Protocol Extensions

- `TargetDescriptionXmlOverride` - Allow targets to override the target description XML file (`target.xml`) specified by `Target::Arch::target_description_xml`. This is useful in cases where a `Target` is expected to be generic over multiple architectures. [\#43](https://github.com/daniel5151/gdbstub/pull/43) (with help from [DrChat](https://github.com/DrChat))

# 0.4.4

#### Bugfixes

- use `write!` instead of `writeln!` in `output!` macro [\#41](https://github.com/daniel5151/gdbstub/issues/41)

# 0.4.3

#### New Arch Implementations

- Implement `RegId` for Mips/Mips64 [\#38](https://github.com/daniel5151/gdbstub/pull/38) ([starfleetcadet75](https://github.com/starfleetcadet75))
- Implement `RegId` for MSP430 [\#38](https://github.com/daniel5151/gdbstub/pull/38) ([starfleetcadet75](https://github.com/starfleetcadet75))

# 0.4.2

#### Packaging

- Exclude test object files from package [\#37](https://github.com/daniel5151/gdbstub/pull/37) ([keiichiw](https://github.com/keiichiw))

# 0.4.1

#### New Arch Implementations

- Implement `RegId` for x86/x86_64 [\#34](https://github.com/daniel5151/gdbstub/pull/34) ([keiichiw](https://github.com/keiichiw))

#### Bugfixes

- Switch fatal error signal from `T06` to `S05`,
- specify cfg-if 0.1.10 or later [\#33](https://github.com/daniel5151/gdbstub/pull/33) ([keiichiw](https://github.com/keiichiw))
  - `cargo build` fails if cfg-if is 0.1.9 or older

#### Internal Improvements

- Don't hard-code u64 when parsing packets (use big-endian byte arrays + late conversion to `Target::Arch::Usize`).

# 0.4.0

This version includes a _major_ API overhaul, alongside a slew of new features and general improvements. While updating to `0.4.0` will require some substantial code modifications, it's well worth the effort, as `0.4.0` is the safest, leanest, and most featureful release of `gdbstub` yet!

Fun fact: Even after adding a _bunch_ of new features and bug-fixes, the in-tree `example_no_std` has remained just as small! The example on the `semver-fix-0.2.2` branch is `20251` bytes, while the example on `0.4.0` is `20246` bytes.

#### Breaking API Changes

- Rewrite the `Target` API in terms of "Inlineable Dyn Extension Traits" (IDETs)
  - _By breaking up `Target` into smaller pieces which can be mixed-and-matched, it not only makes it easier to get up-and-running with `gdbstub`, but it also unlocks a lot of awesome internal optimizations:_
    - Substantially reduces binary-size footprint by guaranteeing dead-code-elimination of parsing/handling unimplemented GDB protocol features.
    - Compile-time enforcement that certain groups of methods are implemented in-tandem (e.g: `add_sw_breakpoint` and `remove_sw_breakpoint`).
- Update the `Target` API with support for non-fatal error handling.
  - _The old approach of only allowing \*fatal\* errors was woefully inadequate when dealing with potentially fallible operations such as reading from unauthorized memory (which GDB likes to do a bunch), or handling non-fatal `std::io::Error` that occur as a result of `ExtendedMode` operations. The new `TargetResult`/`TargetError` result is much more robust, and opens to door to supporting additional error handling extensions (such as LLDB's ASCII Errors)._
- Update the `Connection` trait with new methods (`flush` - required, `write_all`, `on_session_start`)
- Lift `Registers::RegId` to `Arch::RegId`, and introduce new temporary `RegIdImpl` solution for avoiding breaking API changes due to new `RegId` implementations (see [\#29](https://github.com/daniel5151/gdbstub/pull/29))
- Mark various `RegId` enums as `#[non_exhaustive]`, allowing more registers to be added if need be.
- Error types are now marked as `#[non_exhaustive]`.

#### New Protocol Extensions

- `ExtendedMode` - Allow targets to run new processes / attach to existing processes / restart execution.
  - Includes support for `set disable-randomization`, `set environment`, `set startup-with-shell`, and `set cwd` and `cd`.
- `SectionOffsets` - Get section/segment relocation offsets from the target. [\#30](https://github.com/daniel5151/gdbstub/pull/30) ([mchesser](https://github.com/mchesser))
  - Uses the `qOffsets` packet under-the-hood.

#### Bugfixes

- Fix issues related to selecting the incorrect thread after hitting a breakpoint in multi-threaded targets.
- Ensure that `set_nodelay` is set when using a `TcpStream` as a `Connection` (via the new `Connection::on_session_start` API)
  - _This should result in a noticeable performance improvement when debugging over TCP._

#### Internal Improvements

- Removed `btou` dependency.
- Removed all `UTF-8` aware `str` handling code.
  - _GDB uses a pure ASCII protocol, so including code to deal with UTF-8 resulted in unnecessary binary bloat._

# 0.3.0 (formerly 0.2.2)

This version contains a few minor breaking changes from `0.2.1`. These are only surface-level changes, and can be fixed with minimal effort.

Version `0.3.0` is identical to the yanked version `0.2.2`, except that it adheres to `cargo`'s [modified SemVer rule](https://doc.rust-lang.org/cargo/reference/manifest.html#the-version-field) which states that the pre-`0.x.y` breaking changes should still bump the minor version.

Thanks to [h33p](https://github.com/h33p) for reporting this issue ([\#27](https://github.com/daniel5151/gdbstub/issues/27))

#### Breaking API Changes

- Update `Target::resume` API to replace raw `&mut dyn Iterator` with a functionally identical concrete `Actions` iterator.
- Mark the `StopReason` enum as `#[non_exhaustive]`, allowing further types to be added without being considered as an API breaking change.

#### New Protocol Extensions

- Add `Target::read/write_register` support (to support single register accesses) [\#22](https://github.com/daniel5151/gdbstub/pull/22) ([thomashk0](https://github.com/thomashk0))
- Add `StopReason::Signal(u8)` variant, to send arbitrary signal codes [\#19](https://github.com/daniel5151/gdbstub/pull/19) ([mchesser](https://github.com/mchesser))

#### New Arch Implementations

- Add partial RISC-V support (only integer ISA at the moment) [\#21](https://github.com/daniel5151/gdbstub/pull/21) ([thomashk0](https://github.com/thomashk0))
- Add i386 (x86) support [\#23](https://github.com/daniel5151/gdbstub/pull/23) ([jamcleod](https://github.com/jamcleod))
- Add 32-bit PowerPC support [\#25](https://github.com/daniel5151/gdbstub/pull/25) ([jamcleod](https://github.com/jamcleod))

# 0.2.1

#### New Arch Implementations

- Add x86_64 support [\#11](https://github.com/daniel5151/gdbstub/pull/11) ([jamcleod](https://github.com/jamcleod))
- Add Mips and Mips64 support [\#13](https://github.com/daniel5151/gdbstub/pull/13) ([starfleetcadet75](https://github.com/starfleetcadet75))

#### Internal Improvements

- Documentation improvements
  - Document PC adjustment requirements in `Target::resume`
  - Add docs on handling non-fatal invalid memory reads/writes in `Target::read/write_addrs`.

# 0.2.0

_start of changelog_
