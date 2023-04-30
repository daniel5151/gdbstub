#!/bin/bash
set -e

cd "$(dirname "$(realpath "$0")")"

if ! command -v rustfilt &> /dev/null
then
    cargo install rustfilt
fi

rm -rf ./target
cargo rustc --release -- --emit asm -C "llvm-args=-x86-asm-syntax=intel"
cat ./target/release/deps/gdbstub_nostd-*.s | rustfilt > asm.s
sed -i -E '/\.(cfi_def_cfa_offset|cfi_offset|cfi_startproc|cfi_endproc|size)/d' asm.s

if [ -n "$EXTRA_TRIM" ]; then
    sed -i -E '/\.(Ltmp|file|loc)/d' asm.s
    sed -i -E '/.section\t.debug_loc/,$d' asm.s
fi

echo "asm emitted to asm.s"

if grep "core::panicking::panic_fmt" asm.s
then
    echo "found panic in example_no_std!"
    exit 1
else
    echo "no panics in example_no_std"
fi
