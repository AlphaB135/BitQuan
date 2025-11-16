# BitQuan Consensus System - Bitcoin-Style Decentralized Security

## 📋 ภาพรวม

BitQuan ใช้ระบบ consensus แบบ Bitcoin-style ที่กระจายอำนาจอย่างสมบูรณ์:

- ⛓️ **Longest VALID Chain**: ใช้กฎเดียวกับ Bitcoin - chain ที่ยาวที่สุดและถูกต้อง
- 🚫 **No Centralized Control**: ไม่มี checkpoint, emergency override หรือ voting แบบศูนย์กลาง
- 🛡️ **Mathematical Security**: ความปลอดภัยมาจาก proof-of-work และ consensus rules
- ✅ **Transparent Validation**: ทุก block ถูกตรวจสอบด้วยกฎเดียวกัน

---

## 🚀 การทำงาน

### Bitcoin-Style Block Validation

BitQuan ตรวจสอบ blocks ด้วยกฎมาตรฐาน:

1. **Block Header Validation**
   - Timestamp ไม่เกิน 2 ชั่วโมงในอนาคต
   - Difficulty target ถูกต้อง
   - Merkle root ตรงกับ transactions

2. **Coinbase Validation**
   - Coinbase transaction มี input แบบ null prev_txid
   - Script signature มีความยาว 2-100 bytes
   - เฉพาะ transaction แรกเท่านั้นที่เป็น coinbase

3. **Transaction Validation**
   - ทุก signature ถูกตรวจสอบ
   - Block weight ไม่เกิน 4,000,000 WU
   - Fees และ rewards ถูกต้อง

### Longest VALID Chain Rule

```
Chain A: Genesis -> A1 -> A2 -> A3 (VALID)
Chain B: Genesis -> B1 -> B2 (VALID)

ถ้า Chain A มี cumulative work มากกว่า → เลือก Chain A
ถ้า Chain B มี cumulative work มากกว่า → เลือก Chain B
```

**ไม่มี checkpoint override** - ทุก chain ถูกพิจารณาด้วย work เท่านั้น

---

## 🛡️ ความปลอดภัย

### การป้องกัน Attacks

- **51% Attack**: ต้องควบคุม >50% hashrate เหมือน Bitcoin
- **Double Spend**: ป้องกันด้วย confirmations และ longest chain
- **Invalid Blocks**: ถูก reject ทันที ไม่สามารถบังคับให้ accept ได้

### ไม่มี Special Privileges

- ❌ ไม่มี emergency rollback
- ❌ ไม่มี developer checkpoints  
- ❌ ไม่มี manual voting
- ❌ ไม่มี circuit breakers

ทุก node ใช้กฎเดียวกัน ไม่มี operator privileges

---

## ⚙️ Configuration

### Mainnet Configuration

```toml
[consensus]
# Bitcoin-style consensus (no checkpoints)
checkpoint_enabled = false

# Block validation limits
max_block_weight = 4000000
max_transaction_size = 1000000

# Proof of Work
difficulty_adjustment_interval = 2016
target_block_time = 600
```

### Genesis Block

```json
{
  "genesis_hash": "1a3e156469520d4d46dad77241e37651e1c186571d499e332d263876023e2c7b",
  "checkpoint_hashes": [],
  "consensus_params": {
    "target_block_time": 600,
    "max_block_size": 4000000
  }
}
```

**ไม่มี checkpoint_hashes** - chain เริ่มจาก genesis โดยไม่มี intervention

---

## 🧪 Testing

### Consensus Validation Tests

```bash
# Test Bitcoin-style validation
cargo test -p bitquan-consensus validate_block

# Test longest chain rule
cargo test -p bitquan-consensus fork_choice

# Test invalid block rejection
cargo test -p bitquan-consensus reject_invalid_blocks
```

### Fork Choice Tests

```bash
# Test reorganization
cargo test -p bitquan-consensus test_fork_choice_reorg

# Test invalid block handling
cargo test -p bitquan-consensus test_reject_invalid_blocks_in_fork_choice
```

---

## 📚 อ้างอิง

### Core Modules

- 📁 [`crates/consensus/src/lib.rs`](crates/consensus/src/lib.rs) - Main validation logic
- 📁 [`crates/consensus/src/fork.rs`](crates/consensus/src/fork.rs) - Longest chain rule
- 📁 [`crates/consensus/src/pow.rs`](crates/consensus/src/pow.rs) - Proof of work validation

### Configuration

- 📁 [`config/mainnet.toml`](config/mainnet.toml) - Mainnet settings
- 📁 [`genesis/mainnet.json`](genesis/mainnet.json) - Genesis block

### Documentation

- 📁 [`docs/spec/`](docs/spec/) - Technical specifications
- 📁 [`docs/security/`](docs/security/) - Security analysis

---

## 🔍 Monitoring

### Chain Health

```bash
# Check consensus rules
cargo run --bin bitquan-node -- --validate-consensus

# Monitor fork choice
cargo run --bin bitquan-node -- --monitor-forks
```

### Debug Tools

```bash
# Validate specific block
cargo run --bin bitquan-node -- --validate-block <hash>

# Check chain work
cargo run --bin bitquan-node -- --chain-work <tip-hash>
```

---

## ⚠️ ข้อควรระวัง

### ไม่มี Safety Nets

- ไม่มี emergency stop
- ไม่มี manual intervention
- ไม่มี centralized coordination

### Network Security

- ต้องรักษา hashrate ที่เพียงพอ
- ต้องมี node diversity ทางภูมิศาสตร์
- ต้อง monitor network health สม่ำเสมอ

---

## 🎯 สรุป

BitQuan ใช้ Bitcoin-style consensus ที่:

✅ **กระจายอำนาจ** - ไม่มีศูนย์กลางควบคุม  
✅ **คานวณได้** - ทุก node ตรวจสอบด้วยกฎเดียวกัน  
✅ **ปลอดภัย** - ความปลอดภัยจาก mathematics ไม่ใช่คน  
✅ **โปร่งใส** - ไม่มี special privileges หรือ backdoors  

**True Decentralization** - อำนาจอยู่ที่ network ไม่ใช่ developers