# armv4t

An incredibly simple emulator to run elf binaries compiled with `arm-none-eabi-cc -march=armv4t`.

Run `gdb-arm-none-eabi` (or alternatively, `gdb-multiarch`) from the `test_bin` directory to automatically connect to the emulator + load debug symbols for the emulated binary.

This example can be run using:

```bash
cargo run --example armv4t --features=std
```

**NOTE:** If debug symbols couldn't be loaded, try rebuilding `test.elf` locally (requires the `arm-none-eabi` toolchain to be installed), and recompiling the example.

## Unix Domain Sockets

GDB versions since \~2018 support running a debugging session over Unix Domain Sockets (UDS). Debugging over UDS can feel much snappier than debugging over loopback TCP.

Running the example with the `--uds` flag will bind the GdbStub to a socket at `/tmp/armv4t_gdb`.

This feature is only supported on Unix-like systems.
