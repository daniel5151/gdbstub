# Changelog

All notable changes to this project will be documented in this file.

This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

# 0.4.0

This version includes a _major_ API overhaul, alongside a slew of new features and general improvements. While updating to `0.4.0` will require some substantial code modifications, it's well worth the effort, as `0.4.0` is the safest, leanest, and most featureful release of `gdbstub` yet!

#### API Changes

-   Rewrite the `Target` API in terms of "Inlineable Dyn Extension Traits" (IDETs)
    -   _By breaking up `Target` into smaller pieces which can be mixed-and-matched, it not only makes it easier to get up-and-running with `gdbstub`, but it also unlocks a lot of awesome internal optimizations:_
        -   Substantially reduces binary-size footprint by guaranteeing dead-code-elimination of parsing/handling unimplemented GDB protocol features.
        -   Compile-time enforcement that certain groups of methods are implemented in-tandem (e.g: `add_sw_breakpoint` and `remove_sw_breakpoint`).
-   Update the `Target` API with support for non-fatal error handling.
    -   _The old approach of only allowing \*fatal\* errors was woefully inadequate when dealing with potentially fallible operations such as reading from unauthorized memory (which GDB likes to do a bunch), or handling non-fatal `std::io::Error` that occur as a result of `ExtendedMode` operations. The new `TargetResult`/`TargetError` result is much more robust, and opens to door to supporting additional error handling extensions (such as LLDB's ASCII Errors)._
-   Update the `Connection` trait with new methods (`flush` - required, `write_all`, `on_session_start`)
-   Lift `Registers::RegId` to `Arch::RegId`, and introduce new temporary `RegIdImpl` solution for avoiding breaking API changes due to new `RegId` implementations (see [\#29](https://github.com/daniel5151/gdbstub/pull/29))
-   Mark various `RegId` enums as `#[non-exhaustive]`, allowing more registers to be added if need be.
-   Error types are now marked as `#[non-exhaustive]`.

#### New Protocol Extensions

-   `ExtendedMode` - Allow targets to run new processes / attach to existing processes / restart execution.
    -   Includes support for `set disable-randomization`, `set environment`, `set startup-with-shell`, and `set cwd` and `cd`.
-   `SectionOffsets` - Get section/segment relocation offsets from the target. [\#30](https://github.com/daniel5151/gdbstub/pull/30) ([mchesser](https://github.com/mchesser))
    -   Uses the `qOffsets` packet under-the-hood.

#### Bugfixes

-   Fix issues related to selecting the incorrect thread after hitting a breakpoint in multi-threaded targets.
-   Ensure that `set_nodelay` is set when using a `TcpStream` as a `Connection` (via the new `Connection::on_session_start` API)
    -   _This should result in a noticeable performance improvement when debugging over TCP._

#### Misc

-   Removed `btou` dependency.
-   Removed all `UTF-8` aware `str` handling code.
    -   _GDB uses a pure ASCII protocol, so including code to deal with UTF-8 resulted in unnecessary binary bloat._

# 0.3.0 (formerly 0.2.2)

This version contains a few minor breaking changes from `0.2.1`. These are only surface-level changes, and can be fixed with minimal effort.

Version `0.3.0` is identical to the yanked version `0.2.2`, except that it adheres to `cargo`'s [modified SemVer rule](https://doc.rust-lang.org/cargo/reference/manifest.html#the-version-field) which states that the pre-`0.x.y` breaking changes should still bump the minor version.

Thanks to [h33p](https://github.com/h33p) for reporting this issue ([\#27](https://github.com/daniel5151/gdbstub/issues/27))

#### API Changes

-   Update `Target::resume` API to replace raw `&mut dyn Iterator` with a functionally identical concrete `Actions` iterator.
-   Mark the `StopReason` enum as `#[non-exhaustive]`, allowing further types to be added without being considered as an API breaking change.

#### New Protocol Extensions

-   Add `Target::read/write_register` support (to support single register accesses) [\#22](https://github.com/daniel5151/gdbstub/pull/22) ([thomashk0](https://github.com/thomashk0))
-   Add `StopReason::Signal(u8)` variant, to send arbitrary signal codes [\#19](https://github.com/daniel5151/gdbstub/pull/19) ([mchesser](https://github.com/mchesser))

#### New Arch Implementations

-   Add partial RISC-V support (only integer ISA at the moment) [\#21](https://github.com/daniel5151/gdbstub/pull/21) ([thomashk0](https://github.com/thomashk0))
-   Add i386 (x86) support [\#23](https://github.com/daniel5151/gdbstub/pull/23) ([jamcleod](https://github.com/jamcleod))
-   Add 32-bit PowerPC support [\#25](https://github.com/daniel5151/gdbstub/pull/25) ([jamcleod](https://github.com/jamcleod))

# 0.2.1

#### New Arch Implementations

-   Add x86_86 support [\#11](https://github.com/daniel5151/gdbstub/pull/11) ([jamcleod](https://github.com/jamcleod))
-   Add Mips and Mips64 support [\#13](https://github.com/daniel5151/gdbstub/pull/13) ([starfleetcadet75](https://github.com/starfleetcadet75))

#### Misc

-   Documentation improvements
    -   Document PC adjustment requirements in `Target::resume`
    -   Add docs on handling non-fatal invalid memory reads/writes in `Target::read/write_addrs`.

# 0.2.0

_start of changelog_
