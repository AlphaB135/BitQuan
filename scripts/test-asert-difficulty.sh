#!/usr/bin/env bash
set -euo pipefail

echo "🧪 Running Test Suite: ASERT Difficulty Adjustment Validation..."
CC=clang cargo test -p bitquan-consensus --test asert_tests --quiet 2>/dev/null || cargo test -p bitquan-consensus asert --quiet

echo "✅ ASERT Difficulty Adjustment validation PASSED"
