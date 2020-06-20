# armv4t

An incredibly simple emulator which runs binaries compiled with `arm-none-eabi-cc -march=armv4t`.

Run `gdb-multiarch` from the `test_bin` directory to connect to the emulator.

This example can be run using:

```bash
cargo run --example armv4t --features=std
```

**NOTE:** If debug symbols couldn't be loaded, try rebuilding `test.elf` locally (requires the `arm-none-eabi` toolchain to be installed), and recompiling the example.
