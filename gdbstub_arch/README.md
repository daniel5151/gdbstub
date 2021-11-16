# gdbstub_arch

[![](https://img.shields.io/crates/v/gdbstub_arch.svg)](https://crates.io/crates/gdbstub_arch)
[![](https://docs.rs/gdbstub_arch/badge.svg)](https://docs.rs/gdbstub_arch)
[![](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)](./LICENSE)

Community-contributed implementations of `gdbstub::arch::Arch` for various
architectures.

_Note:_ If an architecture is missing from this crate, that does _not_ mean
that it can't be used with `gdbstub`! So-long as there's support for the
target architecture in GDB, it should be fairly straightforward to implement
`Arch` manually.

Please consider upstreaming any missing `Arch` implementations you happen to
implement yourself! Aside from the altruistic motive of improving `gdbstub`,
upstreaming your `Arch` implementation will ensure that it's kept up-to-date
with any future breaking API changes.

**Disclaimer:** These implementations are all community contributions, and
while they are tested (by the PR's author) and code-reviewed, it's not
particularly feasible to write detailed tests for each architecture! If you
spot a bug in any of the implementations, please file an issue / open a PR!
