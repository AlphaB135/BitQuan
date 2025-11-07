# bq-stress - Network Stress Testing Tool

**Last Updated: 2025-01-07**

`bq-stress` is a network load testing and stress testing tool for BitQuan. It generates realistic transaction loads, tests P2P network limits, and validates system behavior under stress.

## Usage

```bash
bq-stress <COMMAND> [OPTIONS]
```

## Commands

### Transaction Load Testing

```bash
# Generate constant TPS load
bq-stress tx-spam \
  --tps 100 \
  --duration 60s \
  --node http://127.0.0.1:28332

# Burst load test
bq-stress tx-burst \
  --count 10000 \
  --batch-size 100 \
  --node http://127.0.0.1:28332

# Ramp-up test (gradual TPS increase)
bq-stress tx-ramp \
  --start-tps 10 \
  --end-tps 500 \
  --duration 300s \
  --node http://127.0.0.1:28332
```

### P2P Network Testing

```bash
# Test peer connection limits
bq-stress p2p-flood \
  --target 127.0.0.1:28333 \
  --connections 1000 \
  --duration 60s

# Message spam test
bq-stress p2p-spam \
  --target 127.0.0.1:28333 \
  --message-type inv \
  --rate 1000/s

# Handshake stress
bq-stress p2p-handshake \
  --target 127.0.0.1:28333 \
  --connections 500 \
  --reconnect
```

### Mempool Testing

```bash
# Fill mempool to capacity
bq-stress mempool-fill \
  --target-size 300MB \
  --tx-size 500 \
  --node http://127.0.0.1:28332

# Test fee market dynamics
bq-stress fee-pressure \
  --min-fee 1 \
  --max-fee 100 \
  --duration 120s \
  --node http://127.0.0.1:28332

# Mempool eviction test
bq-stress mempool-evict \
  --fill-size 500MB \
  --evict-threshold 400MB
```

### Blockchain Stress

```bash
# Generate heavy blocks
bq-stress mine-heavy \
  --block-size 4MB \
  --block-count 100 \
  --template-node http://127.0.0.1:28332

# Chain reorg simulation
bq-stress simulate-reorg \
  --depth 6 \
  --target-node http://127.0.0.1:28332

# Large UTXO set test
bq-stress utxo-bloat \
  --utxo-count 1000000 \
  --address bq1...
```

## Configuration File

Example `stress-config.toml`:

```toml
[network]
nodes = [
    "http://node1.testnet:28332",
    "http://node2.testnet:28332",
    "http://node3.testnet:28332",
]
p2p_targets = [
    "node1.testnet:28333",
    "node2.testnet:28333",
]

[tx_spam]
tps = 100
duration = "60s"
wallet = "~/.bitquan/stress-wallet"
fee_rate = 10

[p2p]
max_connections = 1000
handshake_timeout = "5s"
message_rate = 1000

[mempool]
target_size = "300MB"
min_fee = 1
max_fee = 100

[reporting]
output_dir = "stress-reports"
format = "json"  # json, csv, prometheus
prometheus_push_gateway = "http://localhost:9091"
```

Load config:

```bash
bq-stress --config stress-config.toml tx-spam
```

## Wallets for Testing

```bash
# Generate test wallet with funds
bq-stress wallet-gen \
  --output stress-wallet.keystore \
  --fund-from faucet.testnet

# Create multiple wallets for distributed load
bq-stress wallet-gen-batch \
  --count 100 \
  --output-dir stress-wallets/ \
  --fund-amount 10.0
```

## Monitoring & Reporting

```bash
# Real-time metrics display
bq-stress tx-spam --tps 100 --duration 60s --monitor

# Generate detailed report
bq-stress tx-spam --tps 100 --duration 60s --report report.json

# Export Prometheus metrics
bq-stress tx-spam \
  --tps 100 \
  --duration 60s \
  --prometheus-push http://localhost:9091
```

### Metrics Collected

- **Throughput**: Actual TPS achieved vs. target
- **Latency**: Transaction confirmation times (p50, p95, p99)
- **Success Rate**: % of transactions accepted by mempool
- **Mempool Size**: Bytes and transaction count over time
- **Node Health**: CPU, memory, disk I/O during test
- **P2P Stats**: Connection count, bandwidth, message rates

## Test Scenarios

### Pre-Launch Validation

```bash
# Comprehensive stress test suite
bq-stress suite run \
  --scenario pre-launch \
  --nodes http://node1:28332,http://node2:28332 \
  --duration 3600s \
  --report pre-launch-report.json
```

Scenarios included:
- Sustained 100 TPS for 1 hour
- Burst to 500 TPS for 5 minutes
- Mempool fill to 80% capacity
- 1000 concurrent P2P connections
- Chain reorg simulation (depth 3)

### Daily Smoke Test

```bash
bq-stress suite run --scenario smoke --quick
```

### Long-Running Soak Test

```bash
bq-stress suite run \
  --scenario soak \
  --duration 86400s \
  --nodes http://node:28332
```

## Safety & Best Practices

⚠️ **Important**:
- Only run against **testnet** or dedicated test infrastructure
- Never stress test mainnet
- Coordinate with node operators before testing
- Monitor system resources during tests
- Have rollback plan if test causes issues

### Resource Limits

```bash
# Limit CPU usage
bq-stress --max-cpu 80% tx-spam --tps 100

# Limit memory
bq-stress --max-memory 4GB p2p-flood --connections 1000

# Limit network bandwidth
bq-stress --max-bandwidth 100Mbps tx-spam --tps 200
```

## See Also

- [Load Testing Guide](../testnet/LOAD_TESTING.md)
- [Operations Runbook](../ops/RUNBOOK.md)
- [Testnet Setup](../testnet/README.md)

---

*Updated on: 2025-01-07*
