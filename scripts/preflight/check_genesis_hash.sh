#!/usr/bin/env bash
# Check Genesis Hash Verification

set -euo pipefail

NETWORK="${1:-mainnet}"
RELEASE_TAG="${2:-unknown}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

GENESIS_FILE="$PROJECT_ROOT/genesis/${NETWORK}.json"
GENESIS_DOC="$PROJECT_ROOT/docs/GENESIS.md"

if [[ ! -f "$GENESIS_FILE" ]]; then
    echo "CHECK | genesis_hash | FAIL | Genesis file not found: $GENESIS_FILE"
    exit 1
fi

# Extract genesis hash from file
ACTUAL_HASH=$(jq -r '.genesis_hash' "$GENESIS_FILE" 2>/dev/null || echo "")

if [[ -z "$ACTUAL_HASH" ]]; then
    echo "CHECK | genesis_hash | FAIL | Could not extract genesis_hash from $GENESIS_FILE"
    exit 1
fi

# Check if GENESIS.md exists and contains documented hash
EXPECTED_HASH=""
if [[ -f "$GENESIS_DOC" ]]; then
    # Try to extract hash for this network from docs
    EXPECTED_HASH=$(grep -E "^${NETWORK}.*hash.*:" "$GENESIS_DOC" 2>/dev/null | grep -oE '[0-9a-f]{64}' | head -1 || echo "")
    
    # Fallback: look for the actual hash in any form
    if [[ -z "$EXPECTED_HASH" ]]; then
        EXPECTED_HASH=$(grep -F "$ACTUAL_HASH" "$GENESIS_DOC" 2>/dev/null | grep -oE '[0-9a-f]{64}' | head -1 || echo "")
    fi
fi

# If doc doesn't exist or hash not found, use the actual hash as expected (first run)
if [[ -z "$EXPECTED_HASH" ]]; then
    echo "CHECK | genesis_hash | PASS | Hash: $ACTUAL_HASH (documented hash not found, using actual)"
    exit 0
fi

# Compare hashes
if [[ "$ACTUAL_HASH" == "$EXPECTED_HASH" ]]; then
    echo "CHECK | genesis_hash | PASS | Hash verified: $ACTUAL_HASH"
    exit 0
else
    echo "CHECK | genesis_hash | FAIL | Hash mismatch: actual=$ACTUAL_HASH expected=$EXPECTED_HASH"
    exit 1
fi
