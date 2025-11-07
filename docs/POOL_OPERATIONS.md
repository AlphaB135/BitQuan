# Pool Operations Guide

Complete guide to BitQuan mining pool operations, reward distribution, and miner payouts.

## Table of Contents

- [Overview](#overview)
- [Reward System](#reward-system)
- [Database Schema](#database-schema)
- [Pool Lifecycle](#pool-lifecycle)
- [RPC Endpoints](#rpc-endpoints)
- [Metrics](#metrics)

## Overview

BitQuan's mining pool implements a complete reward engine with:

- **Bitcoin-style halving schedule**: 210,000 block intervals
- **Automatic reward calculation**: Base reward + transaction fees
- **Persistent storage**: SQLite database for blocks, miners, and payouts
- **Real-time tracking**: Metrics integration with Prometheus/Grafana

## Reward System

### Block Rewards

BitQuan follows Bitcoin's halving schedule:

```
Initial reward: 50 BQ (5,000,000,000 satoshis)
Halving interval: 210,000 blocks

Block Range        | Reward per Block
-------------------|------------------
0 - 209,999        | 50.00000000 BQ
210,000 - 419,999  | 25.00000000 BQ
420,000 - 629,999  | 12.50000000 BQ
630,000 - 839,999  |  6.25000000 BQ
...                | ...
```

### Reward Calculation

For each block:

```rust
base_reward = 50_0000_0000 >> (height / 210_000)
transaction_fees = sum(tx_inputs) - sum(tx_outputs)
total_reward = base_reward + transaction_fees
```

### Maturity Period

Rewards require 100 confirmations before they can be spent (same as Bitcoin).

## Database Schema

### Tables

#### `miners`
Tracks accumulated rewards per miner.

```sql
CREATE TABLE miners (
    id TEXT PRIMARY KEY,           -- Miner identifier
    total_reward INTEGER NOT NULL  -- Accumulated satoshis
);
```

#### `blocks`
Records all mined blocks.

```sql
CREATE TABLE blocks (
    hash TEXT PRIMARY KEY,         -- Block hash (hex)
    height INTEGER NOT NULL,       -- Block height
    miner_id TEXT NOT NULL,        -- Miner who found block
    reward INTEGER NOT NULL,       -- Block reward (satoshis)
    timestamp INTEGER NOT NULL     -- Unix timestamp
);
```

#### `payouts`
Tracks payout transactions.

```sql
CREATE TABLE payouts (
    id TEXT PRIMARY KEY,           -- Payout UUID
    miner_id TEXT NOT NULL,        -- Miner receiving payout
    amount INTEGER NOT NULL,       -- Amount (satoshis)
    txid TEXT,                     -- Transaction ID (if sent)
    created_at INTEGER NOT NULL    -- Unix timestamp
);
```

### Indexes

```sql
CREATE INDEX idx_blocks_height ON blocks(height);
CREATE INDEX idx_blocks_miner ON blocks(miner_id);
CREATE INDEX idx_payouts_miner ON payouts(miner_id);
```

## Pool Lifecycle

### 1. Block Discovery

When a miner finds a valid block:

```rust
// 1. Validate PoW
let pow_valid = check_header_pow(&block.header)?;

// 2. Submit to network
let result = submitter.submit(&block, Some("miner_id")).await?;

// 3. If accepted, persist and credit
if accepted {
    let height = chain_state.append_block(&block, hash)?;
    let reward = reward_engine.record_block(&block, hash, height, "miner_id")?;
    
    println!("Block accepted! height={}, reward={:.2} BQ", height, reward / 1e8);
}
```

### 2. Reward Attribution

```rust
// Calculate reward
let base = 50_0000_0000 >> (height / 210_000);
let fees = calculate_transaction_fees(&block);
let total = base + fees;

// Credit miner
pool_db.update_miner_reward("miner_id", total)?;
pool_db.insert_block(&BlockRecord {
    hash: hex::encode(block_hash),
    height,
    miner_id: "miner_id".to_string(),
    reward: total,
    timestamp: block.header.time as u64,
})?;
```

### 3. Payout Processing

Miners can request payouts once rewards mature:

```rust
// Check miner balance
let balance = reward_engine.get_miner_reward("miner_id")?;

// Create payout (manual or automated)
if balance >= minimum_payout {
    let payout_id = reward_engine.record_payout(
        "miner_id",
        balance,
        Some("tx_hash".to_string())
    )?;
    
    // Send on-chain transaction
    send_payout_transaction(miner_address, balance)?;
}
```

## RPC Endpoints

### `getpoolstats`

Get overall pool statistics.

**Request:**
```json
{
    "jsonrpc": "2.0",
    "method": "getpoolstats",
    "params": [],
    "id": 1
}
```

**Response:**
```json
{
    "jsonrpc": "2.0",
    "result": {
        "height": 12345,
        "total_rewards": 500000000000,
        "miner_count": 25,
        "pool_balance": 450000000000,
        "block_count": 12345
    },
    "id": 1
}
```

### `getminerstats`

Get statistics for a specific miner.

**Request:**
```json
{
    "jsonrpc": "2.0",
    "method": "getminerstats",
    "params": ["miner_alpha"],
    "id": 1
}
```

**Response:**
```json
{
    "jsonrpc": "2.0",
    "result": {
        "miner_id": "miner_alpha",
        "total_reward": 50000000000,
        "blocks_mined": 10,
        "recent_blocks": [
            {
                "hash": "000000abc123...",
                "height": 12340,
                "reward": 5000000000,
                "timestamp": 1699564800
            }
        ]
    },
    "id": 1
}
```

### `createpayout`

Create a payout record (requires JWT authentication).

**Request:**
```json
{
    "jsonrpc": "2.0",
    "method": "createpayout",
    "params": {
        "miner_id": "miner_alpha",
        "amount": 10000000000
    },
    "id": 1
}
```

**Response:**
```json
{
    "jsonrpc": "2.0",
    "result": {
        "payout_id": "550e8400-e29b-41d4-a716-446655440000",
        "txid": "a1b2c3d4..."
    },
    "id": 1
}
```

## Metrics

Pool metrics are exported for Prometheus/Grafana:

### Counters

```
stratum_blocks_persisted_total         # Total blocks persisted to chain
stratum_total_rewards_distributed      # Total rewards distributed (satoshis)
stratum_payouts_total                  # Total payouts completed
```

### Gauges

```
stratum_pool_balance_gauge             # Current pool balance (satoshis)
reward_per_block_gauge                 # Current reward per block (satoshis)
```

### Example Queries

**Pool revenue over time:**
```promql
rate(stratum_total_rewards_distributed[1h])
```

**Average reward per block:**
```promql
reward_per_block_gauge / 100000000  # Convert to BQ
```

**Payout rate:**
```promql
rate(stratum_payouts_total[24h])
```

## Example Usage

### Query Pool Stats via RPC

```bash
# Using curl with JWT token
curl -X POST http://localhost:8332 \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $JWT_TOKEN" \
  -d '{
    "jsonrpc": "2.0",
    "method": "getpoolstats",
    "params": [],
    "id": 1
  }'
```

### Query Miner Balance

```bash
curl -X POST http://localhost:8332 \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $JWT_TOKEN" \
  -d '{
    "jsonrpc": "2.0",
    "method": "getminerstats",
    "params": ["my_miner_id"],
    "id": 1
  }'
```

### View Database Directly

```bash
# Connect to pool database
sqlite3 data/pool.db

# Query miner balances
SELECT id, total_reward / 100000000.0 as balance_bq 
FROM miners 
ORDER BY total_reward DESC 
LIMIT 10;

# Query recent blocks
SELECT height, miner_id, reward / 100000000.0 as reward_bq, 
       datetime(timestamp, 'unixepoch') as time
FROM blocks 
ORDER BY height DESC 
LIMIT 20;

# Query payouts
SELECT miner_id, amount / 100000000.0 as amount_bq, 
       datetime(created_at, 'unixepoch') as time
FROM payouts 
ORDER BY created_at DESC 
LIMIT 10;
```

## Security Considerations

1. **JWT Authentication**: All payout endpoints require valid JWT tokens
2. **Maturity Checks**: Enforce 100-block maturity before payouts
3. **Double-Spend Prevention**: Track payout status to prevent duplicate payments
4. **Balance Verification**: Always verify miner balance before creating payouts
5. **Database Backups**: Regular backups of pool database recommended

## Future Enhancements

- Automatic payout threshold configuration
- Multi-signature payout wallets
- PROP (proportional) vs PPLNS payout schemes
- Historical payout reports
- Miner dashboard web interface
- Email/webhook notifications for payouts

## Troubleshooting

### "Insufficient balance" errors

Check miner's actual balance:
```sql
SELECT total_reward FROM miners WHERE id = 'miner_id';
```

### Missing blocks in database

Verify chain synchronization and database connection.

### Incorrect reward calculations

Check block height and ensure halving logic is correct:
```rust
let halvings = height / 210_000;
let expected_reward = 50_0000_0000 >> halvings;
```

---

For more information, see:
- [API Reference](rpc/API_REFERENCE.md)
- [Metrics Guide](METRICS.md)
- [Security Policy](../SECURITY.md)
