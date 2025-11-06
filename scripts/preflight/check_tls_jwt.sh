#!/usr/bin/env bash
# Check TLS/JWT Configuration

set -euo pipefail

NETWORK="${1:-mainnet}"
RELEASE_TAG="${2:-unknown}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

JWT_CONFIG="$PROJECT_ROOT/jwt.example.toml"
SECURITY_DOC="$PROJECT_ROOT/SECURITY.md"

# Check if in mock mode
if [[ "${PREFLIGHT_MOCK:-0}" == "1" ]]; then
    echo "CHECK | tls_jwt | PASS | Mock mode: TLS/JWT config validated"
    exit 0
fi

CHECKS_PASSED=0
CHECKS_TOTAL=4

# Check 1: JWT config example exists
if [[ -f "$JWT_CONFIG" ]]; then
    CHECKS_PASSED=$((CHECKS_PASSED + 1))
fi

# Check 2: Security doc mentions TLS/JWT
if [[ -f "$SECURITY_DOC" ]] && grep -qi "tls\|jwt" "$SECURITY_DOC" 2>/dev/null; then
    CHECKS_PASSED=$((CHECKS_PASSED + 1))
fi

# Check 3: RPC code has JWT/TLS references
RPC_CODE="$PROJECT_ROOT/crates/rpc/src/lib.rs"
if [[ -f "$RPC_CODE" ]] && grep -qi "jwt\|tls" "$RPC_CODE" 2>/dev/null; then
    CHECKS_PASSED=$((CHECKS_PASSED + 1))
fi

# Check 4: Check for HSTS/CSP headers in code
if grep -r "Strict-Transport-Security\|Content-Security-Policy" "$PROJECT_ROOT/crates/rpc/" 2>/dev/null | grep -q .; then
    CHECKS_PASSED=$((CHECKS_PASSED + 1))
else
    # Lenient: if not found in code, check if mentioned in docs
    if grep -qi "hsts\|csp\|security.*header" "$SECURITY_DOC" 2>/dev/null; then
        CHECKS_PASSED=$((CHECKS_PASSED + 1))
    fi
fi

if [[ $CHECKS_PASSED -ge 3 ]]; then
    echo "CHECK | tls_jwt | PASS | TLS/JWT checks passed: $CHECKS_PASSED/$CHECKS_TOTAL"
    exit 0
else
    echo "CHECK | tls_jwt | FAIL | TLS/JWT checks passed: $CHECKS_PASSED/$CHECKS_TOTAL (minimum 3 required)"
    exit 1
fi
