# BitQuan Consensus Usage Guide - Bitcoin-Style Validation

## 📋 ภาพรวม

BitQuan ใช้ Bitcoin-style consensus ที่ไม่มีระบบศูนย์กลาง:

- ⛓️ **Longest VALID Chain**: เลือก chain ที่มี cumulative work สูงสุด
- 🚫 **No Checkpoints**: ไม่มี manual checkpoints หรือ intervention
- 🛡️ **Pure Mathematics**: ความปลอดภัยจาก proof-of-work เท่านั้น
- ✅ **Deterministic**: ทุก node ใช้กฎเดียวกันเสมอ

---

## 🚀 การใช้งาน

### Basic Block Validation

```rust
use bitquan_consensus::{ConsensusEngine, ConsensusParams};

// Create consensus engine
let engine = ConsensusEngine::phase3_defaults();

// Validate block
let result = engine.validate_block(&block, height)?;
println!("Block valid: {:?}", result);
```

### Fork Choice Management

```rust
use bitquan_consensus::fork::ForkChoice;

let mut fork_choice = ForkChoice::new();

// Add genesis
fork_choice.add_genesis(genesis_header)?;

// Add blocks
let (is_new_tip, reorg) = fork_choice.add_block(block_header)?;
if reorg.is_some() {
    println!("Chain reorganization occurred!");
}
```

### Invalid Block Handling

```rust
// Mark block as invalid (consensus failure)
fork_choice.mark_invalid(block_hash, "Invalid signature".to_string());

// Try to add invalid block's child
let result = fork_choice.add_block(child_header);
assert!(result.is_err()); // Should fail
```

---

## 🛡️ การตรวจสอบความปลอดภัย

### Block Header Validation

```rust
// Timestamp validation (2 hour future limit)
if u64::from(header.time) > current_time + 7200 {
    return Err(ConsensusError::InvalidSignature(
        "Block timestamp too far in the future".to_string()
    ));
}

// Merkle root validation
let calculated_merkle = calculate_merkle_root(&transactions)?;
if calculated_merkle != header.merkle_root {
    return Err(ConsensusError::InvalidSignature(
        "Merkle root mismatch".to_string()
    ));
}
```

### Coinbase Transaction Validation

```rust
// Coinbase must have null input
let coinbase_input = &coinbase.inputs[0];
if coinbase_input.prev_txid != [0u8; 32] || coinbase_input.prev_vout != u32::MAX {
    return Err(ConsensusError::InvalidSignature(
        "Invalid coinbase input".to_string()
    ));
}

// Script length limits
if coinbase_input.script_sig.len() < 2 || coinbase_input.script_sig.len() > 100 {
    return Err(ConsensusError::InvalidSignature(
        "Invalid coinbase script length".to_string()
    ));
}
```

---

## ⚙️ Configuration

### Mainnet Setup

```toml
[network]
id = "mainnet"
genesis_hash = "1a3e156469520d4d46dad77241e37651e1c186571d499e332d263876023e2c7b"

[consensus]
# Bitcoin-style (no centralized features)
checkpoint_enabled = false
max_block_weight = 4000000
asert_half_life = 172800

[mempool]
max_size_bytes = 314572800
min_relay_fee_per_wu = 10
```

### Genesis Block

```json
{
  "genesis_hash": "1a3e156469520d4d46dad77241e37651e1c186571d499e332d263876023e2c7b",
  "checkpoint_hashes": [],
  "consensus_params": {
    "target_block_time": 600,
    "max_block_size": 4000000,
    "coinbase_maturity": 100
  }
}
```

---

## 🧪 Testing

### Unit Tests

```bash
# Test block validation
cargo test -p bitquan-consensus validate_block

# Test fork choice
cargo test -p bitquan-consensus fork_choice

# Test invalid block rejection
cargo test -p bitquan-consensus reject_invalid_blocks
```

### Integration Tests

```bash
# Test full consensus flow
cargo test -p bitquan-consensus test_consensus_integration

# Test reorganization
cargo test -p bitquan-consensus test_fork_choice_reorg

# Test longest chain rule
cargo test -p bitquan-consensus test_longest_valid_chain
```

---

## 🔍 Debug Tools

### Block Validation

```bash
# Validate specific block
cargo run --bin bitquan-node -- \
  --validate-block <block-hash> \
  --height <block-height>

# Check chain work
cargo run --bin bitquan-node -- \
  --chain-work <tip-hash>
```

### Fork Analysis

```bash
# Monitor fork choice
cargo run --bin bitquan-node -- \
  --monitor-forks \
  --max-reorg-depth 100

# Analyze reorganization
cargo run --bin bitquan-node -- \
  --analyze-reorg <old-tip> <new-tip>
```

---

## 📊 Monitoring

### Chain Health Metrics

```rust
// Get current chain state
let height = fork_choice.height();
let tip = fork_choice.best_hash();
let chain_work = fork_choice.get_chain_work(&tip)?;

// Check for invalid blocks
let invalid_count = fork_choice.invalid_blocks.len();
```

### Performance Monitoring

```bash
# Benchmark validation
cargo bench -p bitquan-consensus validate_block

# Profile fork choice
cargo bench -p bitquan-consensus fork_choice
```

---

## ⚠️ ข้อควรระวัง

### No Safety Nets

- ❌ ไม่มี emergency rollback
- ❌ ไม่มี manual intervention
- ❌ ไม่มี centralized coordination

### Network Security

- ✅ ต้องรักษา hashrate ที่เพียงพอ
- ✅ ต้องมี node diversity ทางภูมิศาสตร์
- ✅ ต้อง monitor network health สม่ำเสมอ

### Best Practices

```rust
// Always validate blocks before adding
if let Err(e) = engine.validate_block(&block, height) {
    // Mark as invalid if consensus fails
    fork_choice.mark_invalid(block_hash, format!("Validation failed: {}", e));
    return Err(e);
}

// Monitor reorganizations
if let Some(reorg) = reorg_info {
    if reorg.depth() > MAX_SAFE_REORG_DEPTH {
        // Alert on deep reorgs
        alert_system.trigger_alert(&format!("Deep reorg: {} blocks", reorg.depth()));
    }
}
```

---

## 🎯 สรุป

BitQuan consensus ให้:

✅ **True Decentralization** - ไม่มีศูนย์กลางควบคุม  
✅ **Mathematical Security** - ความปลอดภัยจาก proof-of-work  
✅ **Deterministic Rules** - ทุก node ใช้กฎเดียวกัน  
✅ **Transparent Validation** - ไม่มี hidden privileges  

**Bitcoin-Style Consensus** - ทดสอบแล้วว่าปลอดภัยและน่าเชื่อถือ