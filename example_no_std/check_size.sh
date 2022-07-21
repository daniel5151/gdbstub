#!/bin/bash
set -e

cd "$(dirname "$(realpath $0)")"

# checks the size of the resulting --release level binary (that's been stripped)

cargo build --release

cargo bloat --release --split-std -n 100

strip target/release/gdbstub-nostd
size -A -t target/release/gdbstub-nostd
