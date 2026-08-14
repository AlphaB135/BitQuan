#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "============================================================"
echo "🚀 BITQUAN MASTER TEST ORCHESTRATOR — PRE-TESTNET VALIDATION"
echo "============================================================"
echo ""

declare -A RESULTS

run_suite() {
    local name="$1"
    local cmd="$2"
    
    echo "▶️ Running Test Suite: $name..."
    if eval "$cmd"; then
        RESULTS["$name"]="PASS"
        echo "✅ [$name] PASSED"
    else
        RESULTS["$name"]="FAIL"
        echo "❌ [$name] FAILED"
    fi
    echo "------------------------------------------------------------"
}

cd "$PROJECT_ROOT"

# 1. Environment Check
run_suite "Environment Setup" "bash scripts/setup-test-environment.sh"

# 2. Cryptographic Benchmarks & Dilithium5 Verification
run_suite "PQC Speed & TPS Benchmark" "CC=clang cargo test -p bq-crypto --test bench_speed --release -- --nocapture"

# 3. Chaos Engineering Suite (5 Adversarial Scenarios)
run_suite "Chaos Engineering Suite" "CC=clang cargo test -p bitquan-node --test chaos_adversarial_suite -- --nocapture"

# 4. ASERT Difficulty & Consensus
run_suite "ASERT & Consensus Rules" "bash scripts/test-asert-difficulty.sh"

# 5. RPC Security & JWT Authentication
run_suite "RPC Security & JWT" "bash scripts/test-rpc-auth.sh"

# 6. Wallet & Multisig Scheme
run_suite "Wallet & Multisig Scheme" "CC=clang cargo test -p wallet --lib -- --quiet"

# 7. Mempool & Eviction Policy
run_suite "Mempool & Eviction Policy" "CC=clang cargo test -p bitquan-mempool -- --quiet"

# 8. P2P Network, Backpressure & Eclipse
run_suite "P2P Network & Backpressure" "CC=clang cargo test -p bitquan-network -- --quiet"

# 9. Storage Engine & UTXO Index
run_suite "RocksDB Storage Engine" "CC=clang cargo test -p bitquan-storage -- --quiet"

echo ""
echo "============================================================"
echo "📊 BITQUAN MASTER TEST SUITE RESULTS SUMMARY"
echo "============================================================"

TOTAL=0
PASSED=0

for test in "${!RESULTS[@]}"; do
    result="${RESULTS[$test]}"
    TOTAL=$((TOTAL + 1))
    if [ "$result" = "PASS" ]; then
        PASSED=$((PASSED + 1))
        printf "  %-35s : \033[0;32m%s\033[0m\n" "$test" "PASS"
    else
        printf "  %-35s : \033[0;31m%s\033[0m\n" "$test" "FAIL"
    fi
done

echo "------------------------------------------------------------"
echo "Total Suites Run: $TOTAL | Passed: $PASSED | Failed: $((TOTAL - PASSED))"
echo "============================================================"

if [ "$PASSED" -eq "$TOTAL" ]; then
    echo "🎉 ALL TEST SUITES PASSED! System is 100% Ready for Public Testnet!"
    exit 0
else
    echo "❌ SOME SUITES FAILED!"
    exit 1
fi
