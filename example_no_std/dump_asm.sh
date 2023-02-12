#!/bin/bash
set -e

cd "$(dirname "$(realpath "$0")")"

if ! command -v rustfilt &> /dev/null
then
    cargo install rustfilt
fi

cargo rustc --release -- --emit asm -C "llvm-args=-x86-asm-syntax=intel"
cat ./target/release/deps/gdbstub_nostd-*.s | rustfilt > asm.s
sed -i -E '/\.(cfi_def_cfa_offset|cfi_offset|cfi_startproc|cfi_endproc|size)/d' asm.s
echo "asm emitted to asm.s"

if grep "core::panicking::panic_fmt" asm.s
then
    echo "found panic in example_no_std!"
    exit 1
fi
