#!/usr/bin/env bash
set -euo pipefail

echo "🧪 Running Test Suite: RPC Authentication & Security..."
CC=clang cargo test -p bitquan-rpc --quiet

echo "✅ RPC Authentication & Security tests PASSED"
