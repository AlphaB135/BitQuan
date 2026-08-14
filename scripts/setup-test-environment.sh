#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "🔧 Setting up BitQuan test environment..."

# Check Rust installation
if ! command -v cargo &> /dev/null; then
    echo "❌ Rust not found. Install from https://rustup.rs/"
    exit 1
fi

RUST_VERSION=$(rustc --version | awk '{print $2}')
echo "✅ Rust $RUST_VERSION detected"

# Check Clang
if ! command -v clang &> /dev/null; then
    echo "⚠️  Clang recommended for Dilithium PQC C bindings"
fi

# Create test directories
mkdir -p /tmp/bitquan-tests/{clusters,stress,attack,storage,reorg,asert}
echo "✅ Test directories created in /tmp/bitquan-tests"

echo "✅ Environment setup complete!"
