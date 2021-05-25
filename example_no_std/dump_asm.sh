#!/bin/bash
set -e

cargo rustc --release -- --emit asm -C "llvm-args=-x86-asm-syntax=intel"
cat ./target/release/deps/gdbstub_nostd-*.s | c++filt > asm.s
sed -i -E '/\.(cfi_def_cfa_offset|cfi_offset|cfi_startproc|cfi_endproc|size)/d' asm.s
