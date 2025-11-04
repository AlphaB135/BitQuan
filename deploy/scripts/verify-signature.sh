#!/usr/bin/env bash
set -euo pipefail

# BitQuan Signature Verification Script
# Verifies GPG signatures and checksums of release artifacts

DIST_DIR="${1:-dist}"

if [ ! -d "$DIST_DIR" ]; then
    echo "❌ Distribution directory not found: $DIST_DIR"
    exit 1
fi

echo "🔐 Verifying BitQuan release signatures..."

cd "$DIST_DIR"

# Verify checksums
if [ -f "SHA256SUMS.txt" ]; then
    echo "Verifying SHA256 checksums..."
    sha256sum -c SHA256SUMS.txt
    echo "✓ Checksums verified"
else
    echo "⚠️  SHA256SUMS.txt not found"
fi

# Verify GPG signature if present
if [ -f "SHA256SUMS.txt.asc" ]; then
    echo "Verifying GPG signature..."
    gpg --verify SHA256SUMS.txt.asc SHA256SUMS.txt
    echo "✓ GPG signature verified"
else
    echo "⚠️  GPG signature not found (SHA256SUMS.txt.asc)"
fi

echo "✅ Verification complete!"
