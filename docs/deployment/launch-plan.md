# BitQuan Mainnet Launch Configuration

## Launch Timeline

### Phase 1: Pre-Launch (T-7 days)
- [ ] Deploy all 5 bootstrap nodes
- [ ] Configure DNS records
- [ ] Test network connectivity
- [ ] Security audit final check
- [ ] Community announcement

### Phase 2: Launch Day (T-0)
- [ ] Set genesis timestamp
- [ ] Update genesis block hash
- [ ] Enable mainnet configuration
- [ ] Start mining operations
- [ ] Public launch announcement

## Genesis Block Configuration

### Launch Options

#### Option A: Immediate Launch
```bash
# Set genesis to current time
GENESIS_TIMESTAMP=$(date +%s)
echo "Genesis timestamp: $GENESIS_TIMESTAMP"
```

#### Option B: Scheduled Launch
```bash
# Set genesis to specific time (example: 2025-01-01 00:00:00 UTC)
GENESIS_TIMESTAMP=1735689600  # Jan 1, 2025 00:00:00 UTC
echo "Genesis timestamp: $GENESIS_TIMESTAMP"
```

#### Option C: Community Vote Launch
```bash
# Launch when 51% of community votes agree
# Use governance contract for decision
```

### Recommended Launch Time
- **Date:** January 1, 2025
- **Time:** 00:00:00 UTC
- **Unix Timestamp:** 1735689600
- **Reason:** New Year symbolism, global participation

## Update Genesis Configuration

### 1. Update genesis.json
```json
{
  "genesis_timestamp": 1735689600,
  "genesis_block": {
    "timestamp": 1735689600,
    "nonce": 0,
    "bits": 545259519
  },
  "genesis_hash": "<CALCULATED_HASH>"
}
```

### 2. Update constants in code
```rust
// crates/types/src/genesis.rs
pub const GENESIS_TIME: u32 = 1735689600;
pub const GENESIS_NONCE: u64 = <NEW_NONCE>;
pub const GENESIS_HASH: &str = "<NEW_HASH>";
```

### 3. Mine new genesis block
```bash
./target/release/bitquan-node mine-genesis \
  --output genesis-mainnet.json \
  --timestamp 1735689600
```

## Launch Checklist

### Technical Readiness
- [ ] All bootstrap nodes deployed and tested
- [ ] DNS records propagated globally
- [ ] Genesis block mined and verified
- [ ] Configuration files updated
- [ ] Security audit passed
- [ ] Load testing completed

### Community Readiness
- [ ] Mining pools ready
- [ ] Exchanges listed (future)
- [ ] Wallet applications available
- [ ] Documentation complete
- [ ] Support channels open

### Launch Day Tasks
- [ ] Update mainnet.toml with final genesis
- [ ] Start all bootstrap nodes
- [ ] Enable mining operations
- [ ] Monitor network stability
- [ ] Public announcement
- [ ] Community support

## Launch Commands

### Pre-Launch Preparation
```bash
# Build final release
cargo build --release

# Update genesis timestamp
sed -i "s/1729944000/1735689600/g" crates/types/src/genesis.rs

# Mine new genesis block
./target/release/bitquan-node mine-genesis \
  --output genesis-mainnet.json \
  --max-tries 1000000000

# Update mainnet config
cp genesis-mainnet.json genesis/mainnet.json
```

### Launch Execution
```bash
# Start all bootstrap nodes
for i in {1..5}; do
  ssh seed$i.bitquan.network "sudo systemctl start bitquan-node"
done

# Start mining operations
./target/release/bitquan-node mine \
  --network mainnet \
  --pow hybrid \
  --threads 4

# Monitor network
./target/release/bitquan-node run --config config/mainnet.toml
```

## Monitoring During Launch

### Key Metrics to Watch
- **Block propagation time** < 30 seconds
- **Peer connections** > 50 nodes
- **Hashrate** > 1 MH/s
- **Transaction processing** normal
- **Network stability** no forks

### Alert Thresholds
- **No blocks for 10 minutes** - investigate
- **Less than 10 peers** - check connectivity
- **Hashrate drops 50%** - check miners
- **Multiple forks** - check consensus

## Rollback Plan

If critical issues detected:
1. **Stop all nodes** immediately
2. **Announce pause** to community
3. **Fix issues** in hotfix release
4. **Reset genesis** with new timestamp
5. **Relaunch** after fixes

## Communication Plan

### Pre-Launch (7 days before)
- Blog post: "BitQuan Mainnet Launch Date"
- Twitter announcement thread
- Community AMA session
- Exchange notifications

### Launch Day
- Countdown posts (24h, 6h, 1h, 30m, 5m)
- Launch announcement with genesis hash
- Mining guide for participants
- Block explorer link

### Post-Launch
- First block celebration
- Network statistics dashboard
- Community mining competition
- Development roadmap update
