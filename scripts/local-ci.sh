#!/usr/bin/env bash
set -e

echo "=========================================="
echo "🚀 BitQuan Local CI Execution Suite"
echo "=========================================="

export CC=clang
export CXX=clang++
export CARGO_TERM_COLOR=always
export RUSTFLAGS="-D warnings"

echo "1. Checking code formatting..."
cargo fmt --all -- --check
echo "✅ Code format check passed."

echo "2. Running Clippy linter..."
cargo clippy --all-targets -- -D warnings
echo "✅ Clippy check passed."

echo "3. Running Test Suite across all workspace packages..."
cargo test --package bitquan-types --package bq-crypto --package bitquan-consensus --package bitquan-storage --package bitquan-network --package bitquan-rpc --package bitquan-mempool --package bitquan-node --locked
echo "✅ All tests passed cleanly!"

echo "=========================================="
echo "🎉 ALL LOCAL CI CHECKS PASSED SUCCESSFULLY!"
echo "=========================================="
