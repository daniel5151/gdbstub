#!/bin/bash

# Must be run from the project's root directory.

# checks if a certain packet has been dead-code-eliminated from the resulting binary.
# Arg 1: example to build
# Arg 2: packet name

if [ -z "$1" ]; then
    echo "Must pass example name as first argument (e.g: armv4t)"
    exit 1
fi

if [ -z "$2" ]; then
    echo "Must pass packet name as second argument (e.g: qRcmd)"
    exit 1
fi

cargo build --release --example $1 --features="std __dead_code_marker"
strip ./target/release/examples/$1

output=$(strings ./target/release/examples/$1 | sort | grep --color=always "<$2,")

if [[ $output ]]; then
    echo $output
    echo "Dead code NOT eliminated!"
    exit 1
else
    echo "Dead code eliminated."
    exit 0
fi
