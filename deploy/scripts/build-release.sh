#!/usr/bin/env bash
set -euo pipefail

# BitQuan Release Build Script
# Builds reproducible release binaries for Linux and macOS

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
BUILD_DIR="$PROJECT_ROOT/target/release"
DIST_DIR="$PROJECT_ROOT/dist"

echo "🔨 Building BitQuan release binaries..."
echo "Project root: $PROJECT_ROOT"

cd "$PROJECT_ROOT"

# Clean previous builds
rm -rf "$DIST_DIR"
mkdir -p "$DIST_DIR"

# Verify toolchain version
RUST_VERSION=$(rustc --version | awk '{print $2}')
echo "✓ Rust version: $RUST_VERSION"

# Build with reproducible flags
export RUSTFLAGS="-C target-cpu=generic -C codegen-units=1 -C opt-level=3"
export SOURCE_DATE_EPOCH=1

echo "Building all workspace crates..."
cargo build --locked --release --all-features

# Copy binaries to dist
echo "Packaging binaries..."
cp "$BUILD_DIR/bitquan-node" "$DIST_DIR/"
cp "$BUILD_DIR/devnet_sim" "$DIST_DIR/" 2>/dev/null || true
cp "$BUILD_DIR/simple_miner" "$DIST_DIR/" 2>/dev/null || true

# Generate checksums
cd "$DIST_DIR"
sha256sum * > SHA256SUMS.txt

echo "✅ Build complete!"
echo "Binaries and checksums available in: $DIST_DIR"
ls -lh "$DIST_DIR"
