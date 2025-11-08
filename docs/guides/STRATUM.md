# Stratum Mining Protocol

**Last Updated: 2025-01-07**

BitQuan supports Stratum V1 protocol for pool mining. This document describes the protocol implementation and usage.

## Overview

Stratum is a line-based JSON-RPC protocol used for communication between mining pool servers and miners. BitQuan implements Stratum V1 for compatibility with existing mining software.

## Server Setup

```bash
# Start Stratum server
bitquan-node stratum-server \
  --network testnet \
  --stratum-bind 0.0.0.0:3333 \
  --stratum-allow "127.0.0.1,192.168.0.0/16" \
  --stratum-diff 1.0
```

## Supported Methods

### mining.subscribe

Subscribe to mining notifications.

**Request**:
```json
{"id": 1, "method": "mining.subscribe", "params": ["BitQuan/1.0"]}
```

**Response**:
```json
{
  "id": 1,
  "result": [
    ["mining.notify", "subscription_id"],
    "extranonce1",
    4
  ],
  "error": null
}
```

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

Server notification of new work.

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
    true
  ]
}
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

Example pool configuration:

```toml
[stratum]
bind = "0.0.0.0:3333"
difficulty = 1.0
allow_list = ["127.0.0.1", "192.168.0.0/16"]
max_connections = 1000
timeout = 300

[stratum.vardiff]
enabled = true
target_time = 10
min_diff = 0.5
max_diff = 1000
retarget_time = 60
```

## Client Connection

### cgminer

```bash
cgminer -o stratum+tcp://pool.example.com:3333 -u worker1 -p x
```

### bfgminer

```bash
bfgminer -o stratum+tcp://pool.example.com:3333 -u worker1 -p x
```

### cpuminer

```bash
cpuminer -o stratum+tcp://pool.example.com:3333 -u worker1 -p x
```

## Monitoring

Stratum metrics available at `/metrics`:

```
stratum_connections_active{pool="main"} 42
stratum_shares_accepted_total{pool="main"} 1234
stratum_shares_rejected_total{pool="main"} 5
stratum_hashrate_total{pool="main"} 1.5e9
```

## See Also

- [Pool Operations](../POOL_OPERATIONS.md) - Pool setup and management
- [Mining Guide](../README.md) - Testnet mining
- [Dashboard](../DASHBOARD.md) - Pool dashboard

---

*Updated on: 2025-01-07*
