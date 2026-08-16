#!/bin/bash
# Stress Test - Maximum load for 60 seconds
# 🌸 Hermes Stress Testing

RPC="http://testuser:testpass@127.0.0.1:8332"
DURATION=60
LOG="/tmp/stress_results.log"

echo "🔥 Stress Test - ${DURATION}s Maximum Load" | tee "$LOG"
echo "Started: $(date)" | tee -a "$LOG"

# Get baseline
BASELINE_MEM=$(ps aux | grep bitquan-node | grep -v grep | awk '{print $6}' | head -1)
BASELINE_CPU=$(ps aux | grep bitquan-node | grep -v grep | awk '{print $3}' | head -1)
echo "Baseline - Memory: ${BASELINE_MEM}KB, CPU: ${BASELINE_CPU}%" | tee -a "$LOG"

START=$(date +%s)
REQUESTS=0

# Spawn maximum concurrent workers
for worker in {1..50}; do
    (
        while true; do
            NOW=$(date +%s)
            if [ $((NOW - START)) -ge $DURATION ]; then
                break
            fi

            # Mix of operations
            curl -s -X POST "$RPC" -H "Content-Type: application/json" \
              -d '{"jsonrpc":"2.0","method":"getblockcount","params":[],"id":1}' > /dev/null
            curl -s -X POST "$RPC" -H "Content-Type: application/json" \
              -d '{"jsonrpc":"2.0","method":"getbestblockhash","params":[],"id":2}' > /dev/null
            curl -s -X POST "$RPC" -H "Content-Type: application/json" \
              -d '{"jsonrpc":"2.0","method":"getdifficulty","params":[],"id":3}' > /dev/null
        done
    ) &
done

# Monitor during stress
sleep 10
MID_MEM=$(ps aux | grep bitquan-node | grep -v grep | awk '{print $6}' | head -1)
MID_CPU=$(ps aux | grep bitquan-node | grep -v grep | awk '{print $3}' | head -1)
echo "Mid-test - Memory: ${MID_MEM}KB, CPU: ${MID_CPU}%" | tee -a "$LOG"

# Wait for completion
wait

END=$(date +%s)
PEAK_MEM=$(ps aux | grep bitquan-node | grep -v grep | awk '{print $6}' | head -1)
PEAK_CPU=$(ps aux | grep bitquan-node | grep -v grep | awk '{print $3}' | head -1)

echo "" | tee -a "$LOG"
echo "=== Results ===" | tee -a "$LOG"
echo "Duration: $((END - START))s" | tee -a "$LOG"
echo "Peak Memory: ${PEAK_MEM}KB (Δ $((PEAK_MEM - BASELINE_MEM))KB)" | tee -a "$LOG"
echo "Peak CPU: ${PEAK_CPU}%" | tee -a "$LOG"

# Check if node survived
if curl -s -X POST "$RPC" -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"getblockcount","params":[],"id":999}' | grep -q "result"; then
    echo "✅ NODE SURVIVED STRESS TEST" | tee -a "$LOG"
else
    echo "❌ NODE CRASHED" | tee -a "$LOG"
fi
