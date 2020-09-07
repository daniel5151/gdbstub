# checks the size of the resulting --release level binary (that's been stripped)

cargo build --release

cargo bloat --release --split-std -n 100 --filter=gdbstub

strip target/release/gdbstub-nostd
size -A -t target/release/gdbstub-nostd
