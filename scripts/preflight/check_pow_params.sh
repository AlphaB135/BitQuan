#!/usr/bin/env bash
# Check PoW Parameters

set -euo pipefail

NETWORK="${1:-mainnet}"
RELEASE_TAG="${2:-unknown}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

CONSENSUS_CODE="$PROJECT_ROOT/crates/consensus/src/lib.rs"
GENESIS_FILE="$PROJECT_ROOT/genesis/${NETWORK}.json"

if [[ ! -f "$GENESIS_FILE" ]]; then
    echo "CHECK | pow_params | FAIL | Genesis file not found: $GENESIS_FILE"
    exit 1
fi

# Extract PoW algo from genesis
POW_ALGO=$(jq -r '.consensus_params.pow_algo' "$GENESIS_FILE" 2>/dev/null || echo "")

if [[ -z "$POW_ALGO" ]]; then
    echo "CHECK | pow_params | FAIL | Could not extract pow_algo from genesis"
    exit 1
fi

# Validate mainnet restrictions
if [[ "$NETWORK" == "mainnet" ]]; then
    # Mainnet must use sha256d only
    if [[ "$POW_ALGO" != "sha256d" ]]; then
        echo "CHECK | pow_params | FAIL | Mainnet must use sha256d, found: $POW_ALGO"
        exit 1
    fi
    
    # Check if hybrid is forbidden in code
    if [[ -f "$CONSENSUS_CODE" ]]; then
        if grep -q "hybrid.*forbidden\|mainnet.*sha256d.*only" "$CONSENSUS_CODE" 2>/dev/null; then
            echo "CHECK | pow_params | PASS | Mainnet locked to SHA-256d, hybrid forbidden"
            exit 0
        fi
    fi
    
    echo "CHECK | pow_params | PASS | Mainnet using SHA-256d: $POW_ALGO"
    exit 0
else
    # Testnet can use hybrid
    if [[ "$POW_ALGO" == "sha256d" ]] || [[ "$POW_ALGO" == "hybrid" ]]; then
        echo "CHECK | pow_params | PASS | Testnet PoW algo: $POW_ALGO"
        exit 0
    else
        echo "CHECK | pow_params | FAIL | Unknown PoW algo: $POW_ALGO"
        exit 1
    fi
fi
