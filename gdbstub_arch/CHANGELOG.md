All notable changes to this project will be documented in this file.

This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

# 0.2.1

- Removed all remaining instances of `SingleStepGdbBehavior::Unknown` [\#62](https://github.com/daniel5151/gdbstub/pull/95) ([bet4it](https://github.com/bet4it))

# 0.2.0

**Bumps required `gdbstub` version to 0.6.0**.

#### Breaking Arch Changes

- Improved support + fixes for `Msp430` [\#62](https://github.com/daniel5151/gdbstub/pull/62) ([mchesser](https://github.com/mchesser))
- `X86_64CoreRegId`: Change rip size to 8 [\#87](https://github.com/daniel5151/gdbstub/pull/87) ([gz](https://github.com/gz))
- Removed `RegId` template parameters from the following `Arch` implementations:
  - x86/x64
  - MIPS
  - MSP-430

# 0.1.0

**Bumps required `gdbstub` version to 0.5.0**.

- **`gdbstub::arch` has been moved into a separate `gdbstub_arch` crate**
  - _See [\#45](https://github.com/daniel5151/gdbstub/issues/45) for details on why this was done._
- (x86) Break GPRs & SRs into individual fields/variants [\#34](https://github.com/daniel5151/gdbstub/issues/34)
