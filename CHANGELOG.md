# Changelog

All notable changes to this project will be documented in this file.

This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

# 0.2.2

This version contains a few minor breaking changes from `0.2.1`. These are only surface-level changes, and can be fixed with minimal effort.

As outlined in the SemVer specs [section 4.](https://semver.org/#spec-item-4): _Major version zero (0.y.z) is for initial development. Anything MAY change at any time. The public API SHOULD NOT be considered stable._

-   Add `Target::read/write_register` support (to support single register accesses) [\#22](https://github.com/daniel5151/gdbstub/pull/22) ([thomashk0](https://github.com/thomashk0))
-   Update `Target::resume` API to replace raw `&mut dyn Iterator` with a functionally identical concrete `Actions` iterator.
-   Mark the `StopReason` enum as non-exhaustive, allowing further types to be added without being considered "breaking changes"
-   Add `StopReason::Signal(u8)` variant, to send arbitrary signal codes [\#19](https://github.com/daniel5151/gdbstub/pull/19) ([mchesser](https://github.com/mchesser))
-   New `arch` implementations:
    -   Add partial RISC-V support (only integer ISA at the moment) [\#21](https://github.com/daniel5151/gdbstub/pull/21) ([thomashk0](https://github.com/thomashk0))
    -   Add i386 (x86) support [\#23](https://github.com/daniel5151/gdbstub/pull/23) ([jamcleod](https://github.com/jamcleod))
    -   Add 32-bit PowerPC support [\#25](https://github.com/daniel5151/gdbstub/pull/25) ([jamcleod](https://github.com/jamcleod))

# 0.2.1

-   Add x86_86 support [\#11](https://github.com/daniel5151/gdbstub/pull/11) ([jamcleod](https://github.com/jamcleod))
-   Add Mips and Mips64 support [\#13](https://github.com/daniel5151/gdbstub/pull/13) ([starfleetcadet75](https://github.com/starfleetcadet75))
-   Documentation improvements
    -   Document PC adjustment requirements in `Target::resume`
    -   Add docs on handling non-fatal invalid memory reads/writes in `Target::read/write_addrs`.

# 0.2.0

_start of changelog_
