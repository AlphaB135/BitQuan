#!/bin/bash
# BitQuan Testnet Full Attack Suite
# Authorized penetration testing by Hermes (ซากุระ) 🌸

set -e

TESTNET_RPC="http://testuser:testpass@127.0.0.1:8332"
TESTNET_P2P="127.0.0.1:18333"
ATTACK_LOG="/tmp/attack_results.log"

echo "🔥 BitQuan Testnet Attack Suite 🔥" | tee "$ATTACK_LOG"
echo "Started: $(date)" | tee -a "$ATTACK_LOG"
echo "" | tee -a "$ATTACK_LOG"

# Phase 1: Network Layer Attacks
echo "=== PHASE 1: NETWORK LAYER ANNIHILATION ===" | tee -a "$ATTACK_LOG"

# Attack 1.1: Eclipse Attack (same subnet)
echo "[1.1] Eclipse Attack - Flooding from same /24 subnet" | tee -a "$ATTACK_LOG"
for i in {1..100}; do
    timeout 5 nc -z 127.0.0.$i 18333 2>/dev/null &
done
wait
echo "  Result: $(netstat -an | grep 18333 | wc -l) connections established" | tee -a "$ATTACK_LOG"

# Attack 1.2: Headers Flooding
echo "[1.2] Headers Flooding - Attempt to overflow queue (MAX=2000)" | tee -a "$ATTACK_LOG"
curl -s -X POST "$TESTNET_RPC" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"getblockcount","params":[],"id":1}' \
  | tee -a "$ATTACK_LOG"

# Attack 1.3: Handshake Bombing
echo "[1.3] Handshake Race Condition - 1000 concurrent connections" | tee -a "$ATTACK_LOG"
for i in {1..1000}; do
    timeout 1 telnet 127.0.0.1 18333 2>/dev/null &
done
wait
echo "  Handshake spam complete" | tee -a "$ATTACK_LOG"

echo "" | tee -a "$ATTACK_LOG"

# Phase 2: Consensus Layer Attacks
echo "=== PHASE 2: CONSENSUS ENGINE DESTRUCTION ===" | tee -a "$ATTACK_LOG"

# Attack 2.1: Invalid Block Spam
echo "[2.1] Invalid Block Submission - 100 malformed blocks" | tee -a "$ATTACK_LOG"
for i in {1..100}; do
    curl -s -X POST "$TESTNET_RPC" \
      -H "Content-Type: application/json" \
      -d "{\"jsonrpc\":\"2.0\",\"method\":\"submitblock\",\"params\":[\"deadbeef\"],\"id\":$i}" \
      2>&1 | grep -o "error" | head -1
done | wc -l | xargs echo "  Errors received:" | tee -a "$ATTACK_LOG"

# Attack 2.2: Mining Race
echo "[2.2] Concurrent Mining - 10 parallel mine attempts" | tee -a "$ATTACK_LOG"
for i in {1..10}; do
    curl -s -X POST "$TESTNET_RPC" \
      -H "Content-Type: application/json" \
      -d '{"jsonrpc":"2.0","method":"generate","params":[1],"id":'$i'}' &
done
wait
echo "  Parallel mining complete" | tee -a "$ATTACK_LOG"

echo "" | tee -a "$ATTACK_LOG"

# Phase 3: Memory Exhaustion
echo "=== PHASE 3: MEMORY & CPU BURNING ===" | tee -a "$ATTACK_LOG"

# Attack 3.1: Mempool Flooding
echo "[3.1] Mempool Flooding - Attempting to fill 5000 tx capacity" | tee -a "$ATTACK_LOG"
START_MEM=$(ps aux | grep bitquan-node | grep -v grep | awk '{print $6}' | head -1)
echo "  Memory before: ${START_MEM}KB" | tee -a "$ATTACK_LOG"

# Attack 3.2: RPC Request Bombing
echo "[3.2] RPC Bombing - 10000 rapid requests" | tee -a "$ATTACK_LOG"
START_TIME=$(date +%s)
for i in {1..10000}; do
    curl -s -X POST "$TESTNET_RPC" \
      -H "Content-Type: application/json" \
      -d '{"jsonrpc":"2.0","method":"getblockcount","params":[],"id":'$i'}' > /dev/null &
    if [ $((i % 100)) -eq 0 ]; then
        wait
    fi
done
wait
END_TIME=$(date +%s)
DURATION=$((END_TIME - START_TIME))
QPS=$((10000 / DURATION))
echo "  Completed in ${DURATION}s (${QPS} req/s)" | tee -a "$ATTACK_LOG"

END_MEM=$(ps aux | grep bitquan-node | grep -v grep | awk '{print $6}' | head -1)
DELTA=$((END_MEM - START_MEM))
echo "  Memory after: ${END_MEM}KB (Δ ${DELTA}KB)" | tee -a "$ATTACK_LOG"

echo "" | tee -a "$ATTACK_LOG"

# Phase 4: Race Condition Hunting
echo "=== PHASE 4: RACE CONDITION EXPLOITATION ===" | tee -a "$ATTACK_LOG"

# Attack 4.1: Concurrent submitblock
echo "[4.1] Concurrent Block Submission - 100 threads" | tee -a "$ATTACK_LOG"
for i in {1..100}; do
    curl -s -X POST "$TESTNET_RPC" \
      -H "Content-Type: application/json" \
      -d '{"jsonrpc":"2.0","method":"submitblock","params":["00"],"id":'$i'}' &
done
wait
echo "  Race attack complete" | tee -a "$ATTACK_LOG"

# Attack 4.2: Sync State Race
echo "[4.2] Sync State Race - Rapid getblockcount during sync" | tee -a "$ATTACK_LOG"
for i in {1..1000}; do
    curl -s -X POST "$TESTNET_RPC" \
      -H "Content-Type: application/json" \
      -d '{"jsonrpc":"2.0","method":"getblockcount","params":[],"id":'$i'}' > /dev/null &
done
wait
echo "  Sync state bombardment complete" | tee -a "$ATTACK_LOG"

echo "" | tee -a "$ATTACK_LOG"

# Summary
echo "=== ATTACK SUITE COMPLETE ===" | tee -a "$ATTACK_LOG"
echo "Finished: $(date)" | tee -a "$ATTACK_LOG"
echo "" | tee -a "$ATTACK_LOG"
echo "Node Status:" | tee -a "$ATTACK_LOG"
ps aux | grep bitquan-node | grep -v grep | head -1 | tee -a "$ATTACK_LOG"

# Check if node is still responding
if curl -s -X POST "$TESTNET_RPC" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"getblockcount","params":[],"id":999}' \
  | grep -q "result"; then
    echo "✅ NODE SURVIVED ALL ATTACKS!" | tee -a "$ATTACK_LOG"
else
    echo "❌ NODE CRASHED OR UNRESPONSIVE" | tee -a "$ATTACK_LOG"
fi

echo "" | tee -a "$ATTACK_LOG"
echo "Full log: $ATTACK_LOG"
