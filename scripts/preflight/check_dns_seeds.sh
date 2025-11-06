#!/usr/bin/env bash
# Check DNS Seeds Reachability

set -euo pipefail

NETWORK="${1:-mainnet}"
RELEASE_TAG="${2:-unknown}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

DNS_SEEDS_FILE="$PROJECT_ROOT/genesis/dns_seeds.txt"
MIN_THRESHOLD=${DNS_SEED_THRESHOLD:-60}
TIMEOUT=${DNS_TIMEOUT:-2}

if [[ ! -f "$DNS_SEEDS_FILE" ]]; then
    echo "CHECK | dns_seeds | FAIL | DNS seeds file not found: $DNS_SEEDS_FILE"
    exit 1
fi

# Filter seeds for the network
PORT="8333"
if [[ "$NETWORK" == "testnet" ]]; then
    PORT="18333"
    # Extract testnet seeds (lines containing "testnet")
    SEEDS=$(grep -v "^#" "$DNS_SEEDS_FILE" | grep -v "^$" | grep "testnet" || echo "")
else
    # Extract mainnet seeds (lines without "testnet")
    SEEDS=$(grep -v "^#" "$DNS_SEEDS_FILE" | grep -v "^$" | grep -v "testnet" || echo "")
fi

if [[ -z "$SEEDS" ]]; then
    echo "CHECK | dns_seeds | FAIL | No seeds found for network: $NETWORK"
    exit 1
fi

# Check if in mock mode
if [[ "${PREFLIGHT_MOCK:-0}" == "1" ]]; then
    TOTAL=$(echo "$SEEDS" | wc -l | tr -d ' ')
    REACHABLE=$((TOTAL * 80 / 100))  # Mock 80% reachability
    PERCENTAGE=$((REACHABLE * 100 / TOTAL))
    echo "CHECK | dns_seeds | PASS | Mock mode: Reachable: $REACHABLE/$TOTAL ($PERCENTAGE% >= $MIN_THRESHOLD%)"
    exit 0
fi

TOTAL=0
REACHABLE=0

# Check if bq-preflight binary exists
BQ_PREFLIGHT="$PROJECT_ROOT/target/release/bq-preflight"
if [[ -x "$BQ_PREFLIGHT" ]]; then
    # Use Rust binary for better control
    RESULT=$("$BQ_PREFLIGHT" dns-check --network "$NETWORK" --timeout "$TIMEOUT" 2>/dev/null || echo "")
    if [[ -n "$RESULT" ]]; then
        TOTAL=$(echo "$RESULT" | jq -r '.total' 2>/dev/null || echo "0")
        REACHABLE=$(echo "$RESULT" | jq -r '.reachable' 2>/dev/null || echo "0")
    fi
else
    # Fallback: bash implementation
    while IFS= read -r line; do
        [[ -z "$line" ]] && continue
        TOTAL=$((TOTAL + 1))
        
        # Extract domain:port
        SEED=$(echo "$line" | awk '{print $1}')
        DOMAIN=$(echo "$SEED" | cut -d: -f1)
        SEED_PORT=$(echo "$SEED" | cut -d: -f2)
        
        # Quick DNS resolution check (prefer getent/dig)
        if command -v dig &> /dev/null; then
            if dig +time=$TIMEOUT +short "$DOMAIN" A 2>/dev/null | grep -q .; then
                REACHABLE=$((REACHABLE + 1))
            fi
        elif command -v getent &> /dev/null; then
            if timeout $TIMEOUT getent hosts "$DOMAIN" &>/dev/null; then
                REACHABLE=$((REACHABLE + 1))
            fi
        else
            # Fallback: try nc or timeout+bash
            if command -v nc &> /dev/null; then
                if timeout $TIMEOUT nc -zw1 "$DOMAIN" "$SEED_PORT" 2>/dev/null; then
                    REACHABLE=$((REACHABLE + 1))
                fi
            fi
        fi
    done <<< "$SEEDS"
fi

if [[ $TOTAL -eq 0 ]]; then
    echo "CHECK | dns_seeds | FAIL | No seeds to check"
    exit 1
fi

PERCENTAGE=$((REACHABLE * 100 / TOTAL))

if [[ $PERCENTAGE -ge $MIN_THRESHOLD ]]; then
    echo "CHECK | dns_seeds | PASS | Reachable: $REACHABLE/$TOTAL ($PERCENTAGE% >= $MIN_THRESHOLD%)"
    exit 0
else
    echo "CHECK | dns_seeds | FAIL | Reachable: $REACHABLE/$TOTAL ($PERCENTAGE% < $MIN_THRESHOLD%)"
    exit 1
fi
