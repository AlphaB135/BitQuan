# Load Testing Guide

This document describes how to perform load and stress testing on BitQuan nodes using the `bq-stress` tool.

## Overview

The `bq-stress` tool provides synthetic load testing for:
- **RPC endpoints**: Concurrent JSON-RPC requests with latency metrics
- **Stratum pool**: Simulated miner share submissions

Load testing validates performance under stress and identifies bottlenecks before mainnet launch.

## Installation

Build the stress tool:

```bash
cargo build --release -p bq-stress
```

The binary will be at: `target/release/bq-stress`

## Test Scenarios

### Small Load (Development)

For local testing during development:

**RPC Hammer:**
```bash
./target/release/bq-stress rpc-hammer \
  --url http://localhost:8332 \
  --concurrency 8 \
  --duration 30
```

**Pool Shares:**
```bash
./target/release/bq-stress pool-shares \
  --host localhost \
  --port 3333 \
  --miners 10 \
  --qps 5 \
  --duration 30
```

**Expected Results:**
- p95 RPC latency < 100ms
- Share reject rate < 2%
- No crashes or panics

### Medium Load (Testnet)

For testnet stress validation:

**RPC Hammer:**
```bash
./target/release/bq-stress rpc-hammer \
  --url https://testnet.bitquan.org:8332 \
  --concurrency 32 \
  --duration 120 \
  --output artifacts/load/testnet_rpc.json
```

**Pool Shares:**
```bash
./target/release/bq-stress pool-shares \
  --host testnet.bitquan.org \
  --port 3333 \
  --miners 50 \
  --qps 20 \
  --duration 120 \
  --output artifacts/load/testnet_pool.json
```

**Expected Results:**
- p95 RPC latency < 250ms
- Share reject rate < 1.5%
- Graceful 429 rate limit handling
- Orphan rate < 1%

### Large Load (Pre-Mainnet)

For final pre-launch validation:

**RPC Hammer:**
```bash
./target/release/bq-stress rpc-hammer \
  --url https://mainnet.bitquan.org:8332 \
  --concurrency 64 \
  --duration 300 \
  --output artifacts/load/mainnet_rpc.json
```

**Pool Shares:**
```bash
./target/release/bq-stress pool-shares \
  --host mainnet.bitquan.org \
  --port 3333 \
  --miners 200 \
  --qps 50 \
  --duration 300 \
  --output artifacts/load/mainnet_pool.json
```

**Expected Results:**
- p95 RPC latency < 250ms
- Share reject rate < 1.5%
- No memory leaks over 5-minute duration
- Consistent throughput (no degradation)
- Orphan rate < 1%

## Service Level Objectives (SLOs)

Minimum performance targets for mainnet readiness:

| Metric | Target | Critical Threshold |
|--------|--------|-------------------|
| RPC p95 latency | < 250ms | < 500ms |
| RPC p99 latency | < 500ms | < 1000ms |
| Share reject rate | < 1.5% | < 3% |
| Orphan block rate | < 1% | < 2% |
| Memory growth | < 10MB/hour | < 50MB/hour |
| Connection drops | < 0.1% | < 1% |

If metrics exceed **Critical Threshold**, do not proceed to mainnet.

## Output Format

Reports are saved as JSON:

```json
{
  "test_type": "rpc_hammer",
  "duration_secs": 60,
  "total_requests": 3840,
  "successful": 3812,
  "failed": 8,
  "rate_limited": 20,
  "latency_p50_ms": 45.2,
  "latency_p95_ms": 187.5,
  "latency_p99_ms": 312.8,
  "requests_per_sec": 64.0
}
```

## Continuous Testing

Integrate load tests into CI/CD:

```yaml
# .github/workflows/load-test.yml
- name: Run load test
  run: |
    cargo run --release -p bq-stress -- rpc-hammer --concurrency 16 --duration 30
    # Parse artifacts/load/*.json and assert SLOs
```

## Troubleshooting

**High latency (p95 > 500ms):**
- Check CPU/memory usage on node
- Verify network latency (`ping`)
- Review RPC rate limits

**High reject rate (> 3%):**
- Check Stratum difficulty settings
- Verify miner hardware compatibility
- Review pool logs for errors

**Connection drops:**
- Increase `ulimit -n` (file descriptors)
- Check firewall rules
- Verify TLS certificate validity

## Safety Notes

- **Do not run stress tests against production mainnet** without coordination
- Use dedicated testnet/devnet for validation
- Monitor node health during tests (CPU, memory, disk I/O)
- Respect rate limits (429 responses)

## Related Documentation

- [PRELAUNCH_CHECKLIST.md](./PRELAUNCH_CHECKLIST.md) - Pre-mainnet validation gates
- [OBSERVABILITY.md](./OBSERVABILITY.md) - Metrics and dashboards
- [MAINNET_ANNOUNCEMENT.md](./MAINNET_ANNOUNCEMENT.md) - Launch procedures
