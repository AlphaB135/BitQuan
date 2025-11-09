# Network Security Quick Reference - Bitcoin-Style Consensus

Quick reference for BitQuan security procedures. See [EMERGENCY_PROCEDURES.md](EMERGENCY_PROCEDURES.md) for details.

---

## 🚨 Immediate Actions

### 1. Invalid Block Detection
```bash
# Monitor invalid blocks
cargo run --bin bitquan-node -- --monitor-invalid-blocks

# Check consensus health
cargo run --bin bitquan-node -- --consensus-health
```

### 2. Network Partition
```bash
# Check connectivity
cargo run --bin bitquan-node -- --check-connectivity

# Monitor sync status
cargo run --bin bitquan-node -- --sync-status
```

### 3. Hashrate Monitoring
```bash
# Monitor hashrate distribution
cargo run --bin bitquan-node -- --monitor-hashrate

# Check pool concentration
cargo run --bin bitquan-node -- --pool-distribution
```

---

## 🛡️ Security Commands

### Block Validation
```bash
# Validate specific block
cargo run --bin bitquan-node -- --validate-block <hash>

# Check chain work
cargo run --bin bitquan-node -- --chain-work <tip>

# Verify chain integrity
cargo run --bin bitquan-node -- --verify-chain
```

### Fork Monitoring
```bash
# Monitor fork choice
cargo run --bin bitquan-node -- --monitor-forks

# Analyze reorganization
cargo run --bin bitquan-node -- --analyze-reorg <old> <new>

# Check reorg depth
cargo run --bin bitquan-node -- --reorg-depth
```

### Network Health
```bash
# Overall health check
cargo run --bin bitquan-node -- --health-check

# Peer connectivity
cargo run --bin bitquan-node -- --peer-status

# Consensus status
cargo run --bin bitquan-node -- --consensus-status
```

---

## ⚙️ Configuration

### Secure Node Config
```toml
[network]
max_peers = 50
min_peers = 8
bootstrap_nodes = ["node1.bitquan.network:8333"]

[consensus]
checkpoint_enabled = false
max_block_weight = 4000000
max_reorg_depth = 100

[security]
enable_invalid_block_tracking = true
alert_on_deep_reorgs = true
max_safe_reorg_depth = 6
```

### Mining Config
```toml
[mining]
validate_templates = true
monitor_efficiency = true
max_block_size = 4000000
```

---

## 🧪 Testing Commands

### Security Tests
```bash
# Test invalid block rejection
cargo test -p bitquan-consensus reject_invalid_blocks

# Test fork choice
cargo test -p bitquan-consensus fork_choice

# Test reorg handling
cargo test -p bitquan-consensus reorg_handling
```

### Attack Simulation
```bash
# Simulate 51% attack
cargo test -p bitquan-consensus simulate_attack

# Test double spend
cargo test -p bitquan-consensus double_spend

# Test partition recovery
cargo test -p bitquan-consensus partition_recovery
```

---

## 📊 Monitoring

### Real-time Metrics
```bash
# Start monitoring dashboard
cargo run --bin bitquan-node -- --dashboard

# Monitor consensus
cargo run --bin bitquan-node -- --monitor-consensus

# Track invalid blocks
cargo run --bin bitquan-node -- --track-invalid
```

### Alert Configuration
```rust
// Alert thresholds
const MAX_REORG_DEPTH: u64 = 6;
const MAX_INVALID_RATE: f64 = 0.01; // 1%
const MIN_HASHRATE_THRESHOLD: f64 = 0.3; // 30% of normal
```

---

## 🚨 Response Procedures

### Invalid Block Found
1. **Automatic**: Nodes reject block immediately
2. **Monitor**: Check invalid block rate
3. **Alert**: Notify if rate exceeds threshold
4. **Investigate**: Analyze block for attack patterns

### Deep Reorganization
1. **Detect**: Reorg depth > 6 blocks
2. **Alert**: Notify network operators
3. **Monitor**: Watch for continued reorgs
4. **Analyze**: Check for 51% attack indicators

### Hashrate Drop
1. **Monitor**: Track hashrate changes
2. **Alert**: Notify on significant drops
3. **Investigate**: Check for network issues
4. **Coordinate**: Communicate with mining community

---

## 📞 Contact & Resources

### Network Status
- **Dashboard**: `http://localhost:8080/dashboard`
- **API**: `http://localhost:8332/api/health`
- **Logs**: `~/.bitquan/logs/`

### Security Resources
- **Source**: `crates/consensus/src/`
- **Tests**: `crates/consensus/src/tests.rs`
- **Config**: `config/mainnet.toml`

### Community
- **GitHub Issues**: Report security issues
- **Discord**: #security channel
- **Documentation**: `docs/security/`

---

## ⚡ Quick Tips

### Do's
✅ Monitor network health continuously  
✅ Set up alerting for anomalies  
✅ Keep software updated  
✅ Diversify mining pools  
✅ Run full validation nodes  

### Don'ts
❌ Ignore invalid block spikes  
❌ Disable validation for speed  
❌ Run with centralized features  
❌ Trust single data source  
❌ Skip security updates  

---

## 🎯 Key Principles

1. **Mathematical Security**: Trust math, not people
2. **Decentralized Response**: Network protects itself
3. **Transparent Rules**: Everyone knows the rules
4. **No Special Privileges**: No operator overrides
5. **Continuous Monitoring**: Always watch network health

**Bitcoin-Style Security** - Proven, reliable, decentralized