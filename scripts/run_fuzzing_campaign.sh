#!/bin/bash
set -euo pipefail

PROJECT_ROOT="/Volumes/ORICO_EXFAT/BitQuan"
cd "$PROJECT_ROOT"

DURATION_SECONDS=$((24 * 60 * 60))  # 24 hours
TARGETS=(
    "fuzz_transaction"
    "fuzz_crypto"
    "fuzz_block"
    "fuzz_network"
    "fuzz_mempool"
    "fuzz_script"
    "fuzz_wire"
    "fuzz_asert"
    "fuzz_consensus"
    "fuzz_pow"
)

echo "Starting fuzzing campaign for $DURATION_SECONDS seconds..."
echo "Targets: ${TARGETS[@]}"

# Create results directory
mkdir -p fuzz/results
CAMPAIGN_ID=$(date +%Y%m%d_%H%M%S)
RESULTS_DIR="fuzz/results/$CAMPAIGN_ID"
mkdir -p "$RESULTS_DIR"

# Function to run a fuzzer
run_fuzzer() {
    local target=$1
    local duration=$2

    echo "[$(date)] Starting fuzzer: $target"

    mkdir -p "$RESULTS_DIR/$target"
    cd fuzz
    cargo +nightly fuzz run "$target" \
        --release \
        -- \
        -max_total_time="$duration" \
        -print_final_stats=1 \
        -artifact_prefix="$RESULTS_DIR/$target/" \
        > "$RESULTS_DIR/$target.log" 2>&1
    cd "$PROJECT_ROOT"

    local exit_code=$?

    if [ $exit_code -eq 0 ]; then
        echo "[$(date)] ✅ Fuzzer $target completed successfully"
    else
        echo "[$(date)] ❌ Fuzzer $target found issues (exit code: $exit_code)"
    fi

    return $exit_code
}

# Run all fuzzers in parallel with time division
TIME_PER_TARGET=$((DURATION_SECONDS / ${#TARGETS[@]}))
PIDS=()

for target in "${TARGETS[@]}"; do
    run_fuzzer "$target" "$TIME_PER_TARGET" &
    PIDS+=($!)
done

# Wait for all fuzzers to complete
echo "Waiting for all fuzzers to complete..."
FAILED=0
for pid in "${PIDS[@]}"; do
    wait "$pid" || FAILED=$((FAILED + 1))
done

# Generate summary report
echo "Generating fuzzing campaign report..."
cat > "$RESULTS_DIR/SUMMARY.md" << EOF
# Fuzzing Campaign Summary

**Campaign ID**: $CAMPAIGN_ID
**Start Time**: $(date)
**Duration**: 24 hours
**Targets**: ${#TARGETS[@]}

## Results

| Target | Status | Crashes | Log |
|--------|--------|---------|-----|
EOF

for target in "${TARGETS[@]}"; do
    CRASH_COUNT=$(ls "$RESULTS_DIR/$target/" 2>/dev/null | wc -l | tr -d ' ')
    if [ "$CRASH_COUNT" -eq 0 ]; then
        echo "| $target | ✅ PASS | 0 | [log]($target.log) |" >> "$RESULTS_DIR/SUMMARY.md"
    else
        echo "| $target | ❌ FAIL | $CRASH_COUNT | [log]($target.log) |" >> "$RESULTS_DIR/SUMMARY.md"
    fi
done

cat >> "$RESULTS_DIR/SUMMARY.md" << EOF

## Artifacts

Crash artifacts are stored in: \`$RESULTS_DIR/<target>/\`

## Next Steps

1. Review crash artifacts
2. Reproduce crashes in unit tests
3. Fix root causes
4. Re-run fuzzing campaign
EOF

echo ""
echo "=========================================="
echo "Fuzzing Campaign Complete"
echo "=========================================="
echo "Results: $RESULTS_DIR"
echo "Failed fuzzers: $FAILED"
echo ""
cat "$RESULTS_DIR/SUMMARY.md"

exit $FAILED
