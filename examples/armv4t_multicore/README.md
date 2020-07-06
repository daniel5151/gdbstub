# armv4t

An incredibly simple emulator to run elf binaries compiled with `arm-none-eabi-cc -march=armv4t`. Uses a dual-core architecture to show off `gdbstub`'s multi-process support. It's not modeled after any real-world system.

**Note:** The actual emulator's code is pretty sloppy, since it's just a contrived example to show off what `gdbstub` is capable of.

Run `gdb-arm-none-eabi` (or alternatively, `gdb-multiarch`) from the `test_bin` directory to automatically connect to the emulator + load debug symbols for the emulated binary.

This example can be run using:

```bash
cargo run --example armv4t --features=std
```

**NOTE:** If debug symbols couldn't be loaded, try rebuilding `test.elf` locally (requires the `arm-none-eabi` toolchain to be installed), and recompiling the example.

## Memory Map

The entire 32-bit address space is accessible as RAM.

Reading from the magic memory location `0xffff_4200` returns `0xaa` if accessed by the CPU, and `0x55` if accessed by the COP.

## Unix Domain Sockets

GDB versions since \~2018 support running a debugging session over Unix Domain Sockets (UDS). Debugging over UDS can feel much snappier than debugging over loopback TCP.

Running the example with the `--uds` flag will bind the GdbStub to a socket at `/tmp/armv4t_gdb`.

This feature is only supported on Unix-like systems.
