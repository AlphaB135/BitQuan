# Hybrid Stratum Mining Protocol

**Last Updated: 2025-11-09**

BitQuan supports hybrid multi-algorithm mining through Stratum V1 protocol. This document describes the protocol implementation and usage for SHA-256d, RandomX, and Ethash algorithms.

## Overview

BitQuan implements a hybrid mining system that supports multiple Proof-of-Work algorithms simultaneously:

- **SHA-256d** - ASIC-friendly (Bitcoin-style)
- **RandomX** - CPU-friendly, quantum-resistant (Monero-style) 
- **Ethash** - GPU-friendly (Ethereum-style)

The Stratum V1 protocol is extended to support algorithm selection and weighted distribution.

## Server Setup

### Basic Hybrid Server

```bash
# Start hybrid Stratum server with all algorithms
bitquan-node stratum-server \
  --network testnet \
  --stratum-bind 0.0.0.0:3333 \
  --stratum-allow "127.0.0.1,192.168.0.0/16" \
  --stratum-diff 1.0 \
  --hybrid-weights "sha256d:1,ethash:2"
```

### Algorithm-Specific Servers

```bash
# SHA-256d only (ASIC miners)
bitquan-node stratum-server \
  --network testnet \
  --stratum-bind 0.0.0.0:3334 \
  --algorithm sha256d \
  --stratum-diff 1.0

# Ethash only (GPU miners)  
bitquan-node stratum-server \
  --network testnet \
  --stratum-bind 0.0.0.0:3335 \
  --algorithm ethash \
  --stratum-diff 1.0

# RandomX only (CPU miners)
bitquan-node stratum-server \
  --network testnet \
  --stratum-bind 0.0.0.0:3336 \
  --algorithm randomx \
  --stratum-diff 1.0
```

## Supported Methods

### mining.subscribe

Subscribe to mining notifications with algorithm support.

**Request**:
```json
{"id": 1, "method": "mining.subscribe", "params": ["BitQuan-Hybrid/1.0", "sha256d"]}
```

**Parameters**:
- `client_signature` - Client identifier and version
- `algorithm` (optional) - Preferred algorithm: "sha256d", "ethash", "randomx", or "auto"

**Response**:
```json
{
  "id": 1,
  "result": [
    ["mining.notify", "subscription_id"],
    "extranonce1",
    4,
    "sha256d"
  ],
  "error": null
}
```

The fourth element in the result array indicates the assigned algorithm.

### mining.authorize

Authorize a worker.

**Request**:
```json
{"id": 2, "method": "mining.authorize", "params": ["worker1", "password"]}
```

**Response**:
```json
{"id": 2, "result": true, "error": null}
```

### mining.submit

Submit a share.

**Request**:
```json
{
  "id": 3,
  "method": "mining.submit",
  "params": [
    "worker1",
    "job_id",
    "extranonce2",
    "ntime",
    "nonce"
  ]
}
```

**Response**:
```json
{"id": 3, "result": true, "error": null}
```

### mining.notify

Server notification of new work with algorithm-specific data.

```json
{
  "id": null,
  "method": "mining.notify",
  "params": [
    "job_id",
    "prevhash",
    "coinb1",
    "coinb2",
    ["merkle_branch"],
    "version",
    "nbits",
    "ntime",
    true,
    "sha256d",
    "clean_jobs"
  ]
}
```

**Additional Parameters**:
- `algorithm` - The algorithm for this job
- `clean_jobs` - Boolean indicating if previous jobs should be discarded

### mining.algorithm_switch (Extension)

Request algorithm change (optional extension).

**Request**:
```json
{"id": 4, "method": "mining.algorithm_switch", "params": ["ethash"]}
```

**Response**:
```json
{"id": 4, "result": true, "error": null}
```

## Difficulty Adjustment

The pool adjusts difficulty per worker based on hashrate:

- Initial difficulty: 1.0
- Target share time: 10 seconds
- Adjustment every 10 shares

## Error Codes

- `20` - Other/Unknown
- `21` - Job not found
- `22` - Duplicate share
- `23` - Low difficulty share
- `24` - Unauthorized worker
- `25` - Not subscribed

## Configuration

### Hybrid Pool Configuration

```toml
[stratum]
bind = "0.0.0.0:3333"
difficulty = 1.0
allow_list = ["127.0.0.1", "192.168.0.0/16"]
max_connections = 1000
timeout = 300

# Hybrid mining settings
hybrid_enabled = true
algorithm_weights = "sha256d:1,ethash:2"
default_algorithm = "auto"

[stratum.vardiff]
enabled = true
target_time = 10
min_diff = 0.5
max_diff = 1000
retarget_time = 60

# Algorithm-specific difficulty
[stratum.algorithms.sha256d]
difficulty_multiplier = 1.0
max_connections = 500

[stratum.algorithms.ethash]
difficulty_multiplier = 1.5
max_connections = 300

[stratum.algorithms.randomx]
difficulty_multiplier = 0.8
max_connections = 200
```

### Single Algorithm Pool

```toml
[stratum]
bind = "0.0.0.0:3334"
algorithm = "sha256d"
difficulty = 1.0
hybrid_enabled = false
```

## Client Connection

### SHA-256d (ASIC Miners)

```bash
# cgminer for ASICs
cgminer -o stratum+tcp://pool.example.com:3334 -u worker1 -p x --algo sha256d

# bfgminer
bfgminer -o stratum+tcp://pool.example.com:3334 -u worker1 -p x --algo sha256d
```

### Ethash (GPU Miners)

```bash
# PhoenixMiner for GPUs
PhoenixMiner -pool stratum+tcp://pool.example.com:3335 -wal worker1 -proto stratum

# lolMiner
lolMiner --pool pool.example.com:3335 --user worker1 --algo ethash
```

### RandomX (CPU Miners)

```bash
# XMRig for RandomX
xmrig -o pool.example.com:3336 -u worker1 -a rx/0 --donate-level=1

# cpuminer-opt
cpuminer -o stratum+tcp://pool.example.com:3336 -u worker1 -a rx
```

### Hybrid Auto-Detection

```bash
# Auto-detect best algorithm
cgminer -o stratum+tcp://pool.example.com:3333 -u worker1 -p x --algo auto
```

## Monitoring

### Hybrid Metrics

Stratum metrics available at `/metrics`:

```
# Connection metrics
stratum_connections_active{pool="main"} 42
stratum_connections_active{pool="main",algorithm="sha256d"} 15
stratum_connections_active{pool="main",algorithm="ethash"} 20
stratum_connections_active{pool="main",algorithm="randomx"} 7

# Share metrics
stratum_shares_accepted_total{pool="main"} 1234
stratum_shares_accepted_total{pool="main",algorithm="sha256d"} 500
stratum_shares_accepted_total{pool="main",algorithm="ethash"} 600
stratum_shares_accepted_total{pool="main",algorithm="randomx"} 134

stratum_shares_rejected_total{pool="main"} 5
stratum_shares_rejected_total{pool="main",algorithm="sha256d"} 2
stratum_shares_rejected_total{pool="main",algorithm="ethash"} 2
stratum_shares_rejected_total{pool="main",algorithm="randomx"} 1

# Hashrate metrics
stratum_hashrate_total{pool="main"} 1.5e9
stratum_hashrate_total{pool="main",algorithm="sha256d"} 1.0e9
stratum_hashrate_total{pool="main",algorithm="ethash"} 0.4e9
stratum_hashrate_total{pool="main",algorithm="randomx"} 0.1e9

# Algorithm distribution
stratum_algorithm_distribution{pool="main",algorithm="sha256d"} 0.33
stratum_algorithm_distribution{pool="main",algorithm="ethash"} 0.50
stratum_algorithm_distribution{pool="main",algorithm="randomx"} 0.17
```

### Dashboard Integration

The hybrid mining dashboard provides real-time visualization:
- Algorithm hashrate distribution
- Mining efficiency per algorithm  
- Block discovery by algorithm
- Network difficulty trends
- Miner performance analytics

## Algorithm Weights and Distribution

The hybrid system uses weighted round-robin to distribute mining work:

- **Default weights**: `sha256d:1, ethash:2, randomx:1`
- **GPU priority**: Ethash gets 2x weight (GPU-friendly)
- **ASIC/CPU balance**: SHA-256d and RandomX get equal weight
- **Dynamic adjustment**: Weights can be adjusted based on network conditions

### Weight Configuration Examples

```toml
# GPU-focused mining
algorithm_weights = "sha256d:1,ethash:4,randomx:1"

# CPU-focused mining  
algorithm_weights = "sha256d:1,ethash:1,randomx:3"

# ASIC-focused mining
algorithm_weights = "sha256d:4,ethash:1,randomx:1"

# Equal distribution
algorithm_weights = "sha256d:1,ethash:1,randomx:1"
```

## Security Considerations

### Mainnet Restrictions
- **RandomX**: Disabled on mainnet (testnet/devnet only)
- **SHA-256d**: Always enabled (primary consensus)
- **Ethash**: Enabled on all networks

### Network Protection
- **Burst Guard**: Prevents 51% attacks across all algorithms
- **Geographic limits**: 30% max voting power per region
- **Economic safeguards**: Staking and slashing apply to all miners

## See Also

- [Pool Operations](../POOL_OPERATIONS.md) - Pool setup and management
- [Mining Guide](../README.md) - Testnet mining
- [Dashboard](../DASHBOARD.md) - Pool dashboard
- [Hybrid Mining Architecture](../architecture/hybrid-mining.md) - Technical details
- [Network Security](../security/NETWORK_PROTECTION.md) - Security mechanisms

---

*Updated on: 2025-11-09*
