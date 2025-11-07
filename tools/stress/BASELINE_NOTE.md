# P2 Performance Baseline

**Date:** 2025-11-07  
**Branch:** perf/p2-async-optimization

## Baseline Capture Plan

This directory will contain baseline and post-optimization stress test results.

### Files to be generated:
1. `baseline_rpc.txt` - RPC hammer test @ 64 concurrency, 60s
2. `baseline_pool.txt` - Pool shares test @ 100 miners, 40 QPS, 60s
3. `after_rpc.txt` - Post-optimization RPC test @ 64 concurrency, 120s
4. `after_pool.txt` - Post-optimization pool test @ 200 miners, 60 QPS, 120s

### Prerequisites:
- Node running at http://127.0.0.1:28332/rpc
- bq-stress tool compiled and ready

### Commands:
```bash
# Baseline (before optimization)
cargo run -p bq-stress -- rpc-hammer --concurrency 64 --duration 60 \
  --url http://127.0.0.1:28332/rpc > tools/stress/baseline_rpc.txt

cargo run -p bq-stress -- pool-shares --miners 100 --qps 40 --duration 60 \
  > tools/stress/baseline_pool.txt

# After optimization
cargo run -p bq-stress -- rpc-hammer --concurrency 64 --duration 120 \
  --url http://127.0.0.1:28332/rpc > tools/stress/after_rpc.txt

cargo run -p bq-stress -- pool-shares --miners 200 --qps 60 --duration 120 \
  > tools/stress/after_pool.txt
```

## Note

Stress metrics will be captured during actual deployment/testing when node is running.
This baseline commit establishes the framework and documents the process.
