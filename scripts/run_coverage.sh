#!/bin/bash
set -e

# check if cargo-llvm-cov is installed
if ! cargo llvm-cov --version &> /dev/null; then
    echo "cargo-llvm-cov not found. Please install it with 'cargo install cargo-llvm-cov'"
    exit 1
fi

# check if cargo-nextest is installed
if ! cargo nextest --version &> /dev/null; then
    echo "cargo-nextest not found. Please install it with 'cargo install cargo-nextest'"
    exit 1
fi

echo "Running tests with coverage..."
cargo llvm-cov nextest --workspace --all-features --lcov --output-path lcov.info
echo "Coverage report generated at lcov.info"
