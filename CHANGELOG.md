# Changelog

All notable changes to this project will be documented in this file.

This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

# 0.5.0-dev

**This changelog is not final, and is subject to change**

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
- Reworked the `MultiThreadOps::resume` API to be significantly more egonomic and efficient
  - See the [transition guide](https://github.com/daniel5151/gdbstub/blob/dev/0.5/docs/transition_guide.md#new-multithreadopsresume-api) for more details.

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
