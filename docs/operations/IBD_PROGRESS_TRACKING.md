# Initial Block Download (IBD) Progress Tracking

## Overview

Initial Block Download (IBD) is the process by which a new node downloads and validates the entire blockchain from peer nodes. BitQuan provides comprehensive progress tracking for IBD to help operators monitor sync status and diagnose issues.

**Last Updated**: 2026-02-11

---

## IBD States

The sync process goes through the following states:

| State | Description |
|-------|-------------|
| `Idle` | Not syncing, node is up to date |
| `Discovering` | Finding best peer height |
| `DownloadingHeaders` | Downloading block headers from peers |
| `DownloadingBlocks` | Downloading full block data |
| `Synced` | Fully synchronized with network |

---

## Monitoring IBD Progress

### RPC Endpoint: `sync`

Query the current sync status:

```bash
# Using curl
curl -X POST http://localhost:8080 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc": "2.0", "method": "sync", "params": [], "id": 1}'

# Using bitquan-cli
bitquan-cli sync
```

### Response Fields

```json
{
  "status": "DownloadingHeaders",
  "local_height": 45000,
  "best_height": 150000,
  "blocks_behind": 105000,
  "progress": 30.0,
  "syncing": true,
  "last_sync_attempt": 1739235600,
  "sync_errors": 0
}
```

| Field | Type | Description |
|-------|------|-------------|
| `status` | string | Current sync state (see IBD States above) |
| `local_height` | number | Current local blockchain height |
| `best_height` | number | Best known height from peers |
| `blocks_behind` | number | Number of blocks remaining to sync |
| `progress` | float | Sync progress percentage (0.0-100.0) |
| `syncing` | boolean | Whether sync is actively in progress |
| `last_sync_attempt` | number | Unix timestamp of last sync attempt |
| `sync_errors` | number | Count of sync errors encountered |

---

## IBD Flow Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                    IBD Process Flow                              │
└─────────────────────────────────────────────────────────────────┘

    ┌──────────┐     ┌──────────────┐     ┌─────────────────┐
    │   Start  │────▶│ Discovering  │────▶│ Downloading     │
    │   IBD    │     │ Best Height  │     │ Headers         │
    └──────────┘     └──────────────┘     └────────┬────────┘
                                                   │
                                                   ▼
                                          ┌─────────────────┐
                                          │ Downloading     │
                                          │ Blocks          │
                                          └────────┬────────┘
                                                   │
                                                   ▼
                                          ┌─────────────────┐
                                          │     Synced      │
                                          └─────────────────┘

    Progress Tracking:
    ┌────────────────────────────────────────────────────────────┐
    │  SyncProgress {                                           │
    │    status: SyncStatus,                                     │
    │    local_height: u64,       ← Updated as blocks arrive    │
    │    best_height: u64,        ← From peer version messages  │
    │    blocks_behind: u64,      ← best_height - local_height  │
    │    progress: f64,           ← (local / best) * 100        │
    │    sync_errors: u64,        ← Incremented on failures     │
    │  }                                                         │
    └────────────────────────────────────────────────────────────┘
```

---

## Headers-First Sync

BitQuan uses a headers-first synchronization approach:

1. **Download Headers First**: Fetch block headers before full blocks
2. **Validate Headers**: Verify PoW and chain continuity
3. **Download Blocks**: Fetch full block data for validated headers
4. **Process Blocks**: Validate transactions and update UTXO set

### `find_headers_after()` Algorithm

When a peer requests blocks via `getblocks`, the node uses the following algorithm:

1. Search through the provided locator hashes (newest first)
2. Find the first hash that exists in our chain
3. Return up to 2000 headers **after** that point
4. If no locator matches, start from genesis (height 0)

**Location**: `crates/node/src/chainstate.rs:217-267`

---

## Monitoring Best Practices

### 1. Track Sync Errors

High `sync_errors` counts may indicate:
- Network connectivity issues
- Malicious peers sending invalid data
- Local storage problems

```bash
# Watch for increasing error counts
watch -n 5 'bitquan-cli sync | jq ".sync_errors"'
```

### 2. Monitor Progress Rate

Calculate blocks per second to estimate completion time:

```bash
# Sample current height
HEIGHT1=$(bitquan-cli sync | jq ".local_height")
sleep 60
HEIGHT2=$(bitquan-cli sync | jq ".local_height")

# Calculate blocks/second
RATE=$((HEIGHT2 - HEIGHT1))
echo "Sync rate: $RATE blocks/second"

# Estimate remaining time
BEHIND=$(bitquan-cli sync | jq ".blocks_behind")
REMAINING=$((BEHIND / RATE))
echo "Estimated time remaining: $REMAINING seconds"
```

### 3. Verify Peer Connections

Ensure sufficient peers for healthy sync:

```bash
bitquan-cli getnetworkstatus | jq ".peers_connected"
```

Recommended: 8+ connected peers for optimal sync speed.

---

## Troubleshooting

### Sync Stuck at "Discovering"

**Symptoms**: Status remains `Discovering`, no progress

**Possible Causes**:
- No peers connected
- All peers have same/local height
- Network connectivity issues

**Solutions**:
```bash
# Check peer connections
bitquan-cli getnetworkstatus

# Manual peer add if needed
bitquan-cli addnode "peer.address:port" add

# Check network connectivity
ping -c 3 bootstrap.bitquan.network
```

### Slow Sync Progress

**Symptoms**: Progress increases very slowly (<1 block/second)

**Possible Causes**:
- Limited peer bandwidth
- High network latency
- Low disk I/O performance

**Solutions**:
- Add more peers (prefer low-latency peers)
- Check disk I/O: `iostat -x 1`
- Verify sufficient system resources

### High Sync Error Count

**Symptoms**: `sync_errors` increasing steadily

**Possible Causes**:
- Malicious peer sending invalid blocks
- Local database corruption
- Network instability

**Solutions**:
```bash
# Check recent logs for error patterns
journalctl -u bitquand -n 100 | grep -i error

# Restart sync (clears transient errors)
systemctl restart bitquand
```

---

## API Reference

### SyncStatus Enum

```rust
pub enum SyncStatus {
    Idle,              // Not syncing
    Discovering,       // Finding best height
    DownloadingHeaders, // Headers phase
    DownloadingBlocks, // Blocks phase
    Synced,           // Fully synced
}
```

### SyncProgress Struct

```rust
pub struct SyncProgress {
    pub status: SyncStatus,
    pub local_height: u64,
    pub best_height: u64,
    pub blocks_behind: u64,
    pub progress: f64,
    pub last_sync_attempt: u64,
    pub sync_errors: u64,
}
```

---

## Related Documentation

- **[P2P Protocol](./P2P_PROTOCOL.md)** - Peer-to-peer message formats
- **[Monitoring](./MONITORING.md)** - General node monitoring guide
- **[Security](./security/SECURITY_STANDARDS.md)** - Security best practices

---

## Implementation Details

**Key Files**:
- `crates/network/src/sync.rs` - Core sync logic and `SyncProgress`
- `crates/network/src/async_sync.rs` - Async sync manager
- `crates/node/src/chainstate.rs` - `find_headers_after()` implementation
- `crates/node/src/rpc.rs` - RPC `sync()` endpoint handler

**Issues**:
- #105 - `find_headers_after()` implementation
- #114 - Phase A IBD bug fixes
- #118 - IBD progress tracking documentation
