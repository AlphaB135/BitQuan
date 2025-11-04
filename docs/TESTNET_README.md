# BitQuan Testnet & Hybrid Mining Guide

This document provides detailed instructions for running BitQuan testnet nodes with hybrid Proof-of-Work mining.

## ⚠️ Network Restrictions

| Network  | SHA-256d | RandomX | Hybrid Mode |
|----------|----------|---------|-------------|
| Mainnet  | ✅ Always | ❌ Never  | ❌ Never     |
| Testnet  | ✅ Yes    | ✅ Yes    | ✅ Yes       |
| Devnet   | ✅ Yes    | ✅ Yes    | ✅ Yes       |
| Regtest  | ✅ Yes    | ✅ Yes    | ✅ Yes       |

**Mainnet uses SHA-256d exclusively.** RandomX is strictly forbidden at the consensus level for maximum security and ASIC compatibility.

## Building with RandomX Support

```bash
# Standard build (SHA-256d only)
cargo build --release

# Build with RandomX support (adds ~50MB dependencies)
cargo build --release --features randomx

# Verify feature compilation
cargo test --features randomx --test hybrid_miner
```

## Hybrid Mining Modes

### 1. Pure SHA-256d (Default)

```bash
./target/release/bitquan-node mine \
  --network testnet \
  --pow hashcash \
  --threads 4 \
  --datadir ./data/testnet
```

### 2. Pure RandomX

```bash
./target/release/bitquan-node mine \
  --network testnet \
  --pow randomx \
  --randomx-mode fast \
  --threads 8 \
  --datadir ./data/testnet
```

**RandomX Memory Requirements:**
- `--randomx-mode fast`: ~256-512MB per thread (faster init, lower perf)
- `--randomx-mode full`: ~2GB per thread (slower init, optimal perf)

### 3. Hybrid Mode (Weighted Mix)

```bash
./target/release/bitquan-node mine \
  --network devnet \
  --pow hybrid \
  --hybrid-weights "sha256d:1,randomx:3" \
  --threads 4 \
  --limit-blocks 50
```

**Weight Interpretation:**
- `sha256d:1,randomx:3` → 25% SHA-256d, 75% RandomX
- Algorithm selection uses weighted round-robin
- Weights are floating-point (e.g., `sha256d:0.5,randomx:1.5`)

## CLI Reference

### Hybrid-Specific Flags

| Flag | Description | Default | Example |
|------|-------------|---------|---------|
| `--pow` | PoW mode: `hashcash`, `randomx`, `hybrid`, `mock` | `hashcash` | `--pow hybrid` |
| `--hybrid-weights` | Algorithm weights (comma-separated) | `sha256d:1,randomx:2` | `--hybrid-weights "sha256d:2,randomx:1"` |
| `--randomx-mode` | RandomX cache mode: `fast` or `full` | `fast` | `--randomx-mode full` |
| `--randomx-seed` | RandomX initialization seed (hex) | Genesis hash | `--randomx-seed deadbeef...` |
| `--threads` | Mining threads (0 = CPU count) | `1` | `--threads 8` |
| `--limit-blocks` | Stop after N blocks mined | None | `--limit-blocks 100` |

### General Mining Flags

| Flag | Description | Default |
|------|-------------|---------|
| `--network` | Network: `mainnet`, `testnet`, `devnet`, `regtest` | `mainnet` |
| `--datadir` | Blockchain storage directory | `./data/chainstate` |
| `--payout-script-hex` | Coinbase payout script (hex) | `76a9140088ac` |
| `--bits` | Override difficulty target (0 = auto) | `0` |
| `--max-nonce` | Max nonce per block attempt | `100000000` |

## Prometheus Metrics

Hybrid miner exposes detailed per-algorithm metrics:

### Metrics Keys

```prometheus
# Total blocks mined per algorithm
pow_mined_blocks_total{algo="sha256d|randomx"}

# Total hash attempts per algorithm  
pow_hash_attempts_total{algo="sha256d|randomx"}

# PoW verification failures (should be rare)
pow_verify_failures_total{algo="sha256d|randomx"}

# Estimated hashrate (hashes/sec)
pow_hashrate_gauge{algo="sha256d|randomx"}

# Average block time (seconds)
pow_block_time_seconds{algo="sha256d|randomx"}
```

### Example Queries

```bash
# Get all metrics
curl http://localhost:9090/metrics | grep pow_

# Blocks mined by RandomX
curl -s http://localhost:9090/metrics | grep 'pow_mined_blocks_total{algo="randomx"}'

# Compare hashrates
curl -s http://localhost:9090/metrics | grep pow_hashrate_gauge
```

## Troubleshooting

### Error: "RandomX disabled on mainnet"

**Cause:** Attempted to use `--pow randomx` or `--pow hybrid` on mainnet.

**Solution:** Mainnet only supports SHA-256d. Use testnet/devnet for hybrid mining.

```bash
# ❌ Wrong
./bitquan-node mine --network mainnet --pow hybrid

# ✅ Correct
./bitquan-node mine --network testnet --pow hybrid
```

### Error: "feature randomx is not enabled"

**Cause:** Binary was built without RandomX support.

**Solution:** Rebuild with feature flag:

```bash
cargo build --release --features randomx
```

## Stratum Mining Server

BitQuan supports running as a Stratum V1 mining pool for external miners.

### Starting Stratum Server

```bash
# Basic Stratum server
./target/release/bitquan-node stratum-server \
  --network testnet \
  --stratum-bind 0.0.0.0:3333 \
  --stratum-diff 1.0

# With IP allowlist
./target/release/bitquan-node stratum-server \
  --network devnet \
  --stratum-bind 0.0.0.0:3333 \
  --stratum-allow "127.0.0.1,192.168.1.0/24,10.0.0.0/8" \
  --stratum-diff 2.0
```

### Connecting Miners

**cgminer (SHA-256d)**:
```bash
cgminer -o stratum+tcp://your-server:3333 \
  -u worker1 \
  -p x \
  --algo sha256d
```

**xmrig (RandomX, testnet only)**:
```bash
xmrig -o your-server:3333 \
  -u worker2 \
  -p x \
  --randomx
```

### Stratum Metrics

Monitor pool activity via Prometheus:

```bash
# Active miners
curl -s http://localhost:9090/metrics | grep stratum_active_miners

# Accepted shares
curl -s http://localhost:9090/metrics | grep 'stratum_shares_total{status="ok"}'

# Rejected shares
curl -s http://localhost:9090/metrics | grep 'stratum_shares_total{status="reject"}'

# Total connections
curl -s http://localhost:9090/metrics | grep stratum_connections_total
```

### Stratum Protocol Support

**Supported Methods**:
- `mining.subscribe` - Initial connection
- `mining.authorize` - Miner authentication
- `mining.submit` - Share submission

**Response Format**: Standard JSON-RPC 2.0

**Share Verification**: Validates nonce against difficulty target for selected algorithm.

---

**Remember**: Hybrid PoW is for testnet experimentation only. Mainnet remains SHA-256d to ensure maximum security, ASIC compatibility, and battle-tested consensus.
