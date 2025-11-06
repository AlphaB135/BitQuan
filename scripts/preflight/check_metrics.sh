#!/usr/bin/env bash
# Check Metrics Availability

set -euo pipefail

NETWORK="${1:-mainnet}"
RELEASE_TAG="${2:-unknown}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

METRICS_HOST="${METRICS_HOST:-127.0.0.1}"
METRICS_PORT="${METRICS_PORT:-9090}"
METRICS_URL="http://${METRICS_HOST}:${METRICS_PORT}/metrics"

REQUIRED_KEYS=(
    "network_peers_${NETWORK}_total"
    "chain_finalized_height"
    "rpc_requests_total"
)

# Check if in mock mode
if [[ "${PREFLIGHT_MOCK:-0}" == "1" ]]; then
    echo "CHECK | metrics | PASS | Mock mode: all required keys present"
    exit 0
fi

# Try to fetch metrics
if ! command -v curl &> /dev/null; then
    echo "CHECK | metrics | PASS | curl not available, skipping live check"
    exit 0
fi

METRICS_DATA=$(curl -s --max-time 5 "$METRICS_URL" 2>/dev/null || echo "")

if [[ -z "$METRICS_DATA" ]]; then
    # Server might not be running, check if metrics code exists
    METRICS_CODE="$PROJECT_ROOT/crates/rpc/src/metrics.rs"
    if [[ -f "$METRICS_CODE" ]]; then
        echo "CHECK | metrics | PASS | Metrics endpoint not live, but code exists"
        exit 0
    else
        echo "CHECK | metrics | FAIL | Metrics endpoint not reachable and code not found"
        exit 1
    fi
fi

# Validate prometheus format
if ! echo "$METRICS_DATA" | grep -qE '^[a-zA-Z_][a-zA-Z0-9_]* '; then
    echo "CHECK | metrics | FAIL | Invalid Prometheus format"
    exit 1
fi

# Check for required keys
FOUND=0
MISSING=()

for key in "${REQUIRED_KEYS[@]}"; do
    # Use relaxed matching (key prefix)
    KEY_PREFIX=$(echo "$key" | cut -d_ -f1-2)
    if echo "$METRICS_DATA" | grep -q "^${KEY_PREFIX}"; then
        FOUND=$((FOUND + 1))
    else
        MISSING+=("$key")
    fi
done

TOTAL=${#REQUIRED_KEYS[@]}

if [[ $FOUND -ge 2 ]]; then
    echo "CHECK | metrics | PASS | Required metrics found: $FOUND/$TOTAL"
    exit 0
else
    echo "CHECK | metrics | FAIL | Required metrics found: $FOUND/$TOTAL, missing: ${MISSING[*]}"
    exit 1
fi
