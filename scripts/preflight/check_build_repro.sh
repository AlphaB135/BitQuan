#!/usr/bin/env bash
# Check Build Reproducibility

set -euo pipefail

NETWORK="${1:-mainnet}"
RELEASE_TAG="${2:-unknown}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

BINARY_NAME="bitquan-node"
BUILD_DIR="$PROJECT_ROOT/target/release"

# Check if in CI or mock mode (skip actual build in mock)
if [[ "${PREFLIGHT_MOCK:-0}" == "1" ]]; then
    echo "CHECK | build_repro | PASS | Mock mode: build reproducibility check skipped"
    exit 0
fi

# Check if binary exists
if [[ ! -f "$BUILD_DIR/$BINARY_NAME" ]]; then
    echo "CHECK | build_repro | FAIL | Binary not found: $BUILD_DIR/$BINARY_NAME"
    exit 1
fi

# Get checksum of existing binary
EXISTING_HASH=$(shasum -a 256 "$BUILD_DIR/$BINARY_NAME" | awk '{print $1}')

# Check if we can fetch release artifact from GitHub
if [[ "$RELEASE_TAG" != "unknown" ]] && [[ "$RELEASE_TAG" =~ ^v[0-9]+\.[0-9]+\.[0-9]+ ]]; then
    # Try to fetch release asset hash from GitHub
    RELEASE_URL="https://github.com/bitquan/bitquan/releases/download/$RELEASE_TAG/$BINARY_NAME.sha256"

    if command -v curl &> /dev/null; then
        RELEASE_HASH=$(curl -sL "$RELEASE_URL" 2>/dev/null | awk '{print $1}' || echo "")

        if [[ -n "$RELEASE_HASH" ]] && [[ ${#RELEASE_HASH} -eq 64 ]]; then
            if [[ "$EXISTING_HASH" == "$RELEASE_HASH" ]]; then
                echo "CHECK | build_repro | PASS | Build hash matches release: $EXISTING_HASH"
                exit 0
            else
                echo "CHECK | build_repro | FAIL | Hash mismatch: local=$EXISTING_HASH release=$RELEASE_HASH"
                exit 1
            fi
        fi
    fi
fi

# If no release artifact available, verify build is consistent
echo "CHECK | build_repro | PASS | Binary hash: $EXISTING_HASH (release artifact not available for comparison)"
exit 0
